use crate::errors::EconomicsError;
use crate::types::{
    DistributionResult, EconomicsParams, MicroIPN, Participation, ParticipationSet, Payouts, Role,
    REPUTATION_SCORE_MAX, REPUTATION_SCORE_MIN, REPUTATION_SCORE_SCALE,
};
use std::collections::{HashMap, HashSet};

/// Apply the fee cap defined in the economics parameters. Returns `(allowed_fees, capped_amount)`.
pub fn apply_fee_cap(
    fees_micro: MicroIPN,
    emission_micro: MicroIPN,
    params: &EconomicsParams,
) -> (MicroIPN, MicroIPN) {
    if params.fee_cap_denom == 0 {
        return (0, fees_micro);
    }

    let max_fees = emission_micro.saturating_mul(params.fee_cap_numer as MicroIPN)
        / params.fee_cap_denom as MicroIPN;
    let allowed = fees_micro.min(max_fees);
    let capped = fees_micro.saturating_sub(allowed);
    (allowed, capped)
}

/// Estimate a contribution multiplier (basis points) derived from observed participation.
pub fn calculate_contribution_multiplier(participation: &Participation) -> u32 {
    let total_blocks = participation
        .blocks_proposed
        .saturating_add(participation.blocks_verified);

    match total_blocks {
        0 => 0,
        1 => 800,
        2..=5 => 1_000,
        6..=10 => 1_200,
        _ => 1_500,
    }
}

/// Validate basic invariants for a participation set.
pub fn validate_participation_set(participants: &ParticipationSet) -> Result<(), EconomicsError> {
    let mut seen = HashSet::new();

    for participant in participants {
        if participant.stake_weight == 0 {
            return Err(EconomicsError::InvalidParticipation(
                "stake weight must be greater than zero".to_string(),
            ));
        }

        if participant.reputation_score_micros < REPUTATION_SCORE_MIN
            || participant.reputation_score_micros > REPUTATION_SCORE_MAX
        {
            return Err(EconomicsError::InvalidParticipation(
                "reputation score must be between 0.1 and 10.0".to_string(),
            ));
        }

        if !seen.insert(participant.validator_id) {
            return Err(EconomicsError::DuplicateValidator(participant.validator_id));
        }
    }

    Ok(())
}

/// Distribute round rewards and capped fees across participating validators.
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    participants: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, DistributionResult), EconomicsError> {
    validate_participation_set(participants)?;

    let (allowed_fees, fee_cap_applied) = apply_fee_cap(fees_micro, emission_micro, params);
    let mut payouts: Payouts = HashMap::new();

    if participants.is_empty() {
        let details = DistributionResult {
            round: 0,
            total_emission_micro: emission_micro,
            total_fees_micro: fees_micro,
            fee_cap_applied_micro: fee_cap_applied,
            net_emission_micro: emission_micro.saturating_add(allowed_fees),
            payouts: HashMap::new(),
            proposer_rewards_micro: 0,
            verifier_rewards_micro: 0,
        };
        return Ok((payouts, emission_micro, details));
    }

    let total_bps =
        (params.proposer_weight_bps as u64 + params.verifier_weight_bps as u64) as MicroIPN;
    if total_bps == 0 {
        return Err(EconomicsError::InvalidParameter(
            "reward weights must sum to a positive value",
        ));
    }

    let mut participant_weights = Vec::with_capacity(participants.len());
    let mut proposer_total: MicroIPN = 0;
    let mut verifier_total: MicroIPN = 0;

    for participant in participants {
        let (proposer_weight, verifier_weight) = compute_role_weights(participant);
        accumulate_weight(&mut proposer_total, proposer_weight);
        accumulate_weight(&mut verifier_total, verifier_weight);
        participant_weights.push((participant.validator_id, proposer_weight, verifier_weight));
    }

    if proposer_total == 0 && verifier_total == 0 {
        return Err(EconomicsError::NoParticipants);
    }

    let mut proposer_pool =
        emission_micro.saturating_mul(params.proposer_weight_bps as MicroIPN) / total_bps;
    let mut verifier_pool = emission_micro.saturating_sub(proposer_pool);

    if proposer_total == 0 {
        verifier_pool = verifier_pool.saturating_add(proposer_pool);
        proposer_pool = 0;
    }
    if verifier_total == 0 {
        proposer_pool = proposer_pool.saturating_add(verifier_pool);
        verifier_pool = 0;
    }

    let fee_pool = if verifier_total == 0 { 0 } else { allowed_fees };
    let net_emission = emission_micro.saturating_add(allowed_fees);

    let mut proposer_paid: MicroIPN = 0;
    let mut verifier_paid: MicroIPN = 0;

    for (validator, proposer_weight, verifier_weight) in participant_weights {
        let mut reward: MicroIPN = 0;

        if proposer_weight > 0 && proposer_total > 0 {
            let share = proposer_pool.saturating_mul(proposer_weight) / proposer_total;
            reward = reward.saturating_add(share);
            proposer_paid = proposer_paid.saturating_add(share);
        }

        if verifier_weight > 0 && verifier_total > 0 {
            let emission_share = verifier_pool.saturating_mul(verifier_weight) / verifier_total;
            let fee_share = if fee_pool > 0 {
                fee_pool.saturating_mul(verifier_weight) / verifier_total
            } else {
                0
            };
            let verifier_total_share = emission_share.saturating_add(fee_share);
            reward = reward.saturating_add(verifier_total_share);
            verifier_paid = verifier_paid.saturating_add(verifier_total_share);
        }

        if reward > 0 {
            payouts.insert(validator, reward);
        }
    }

    let details = DistributionResult {
        round: 0,
        total_emission_micro: emission_micro,
        total_fees_micro: fees_micro,
        fee_cap_applied_micro: fee_cap_applied,
        net_emission_micro: net_emission,
        payouts: payouts.clone(),
        proposer_rewards_micro: proposer_paid,
        verifier_rewards_micro: verifier_paid,
    };

    Ok((payouts, emission_micro, details))
}

fn accumulate_weight(total: &mut MicroIPN, value: MicroIPN) {
    if value > 0 {
        *total = total.saturating_add(value);
    }
}

fn compute_role_weights(participation: &Participation) -> (MicroIPN, MicroIPN) {
    let multiplier = calculate_contribution_multiplier(participation) as MicroIPN;
    if multiplier == 0 {
        return (0, 0);
    }

    let reputation = reputation_factor(participation.reputation_score_micros);
    let base = participation
        .stake_weight
        .saturating_mul(multiplier)
        .saturating_mul(reputation)
        / 1_000; // normalize multiplier basis points

    if base == 0 {
        return (0, 0);
    }

    let proposer_blocks = participation.blocks_proposed.max(1) as MicroIPN;
    let verifier_blocks = participation.blocks_verified.max(1) as MicroIPN;

    let proposer_weight = match participation.role {
        Role::Proposer => base.saturating_mul(proposer_blocks),
        Role::Verifier => 0,
        Role::Both => base.saturating_mul(proposer_blocks),
    };

    let verifier_weight = match participation.role {
        Role::Proposer => 0,
        Role::Verifier => base.saturating_mul(verifier_blocks),
        Role::Both => base.saturating_mul(verifier_blocks),
    };

    (proposer_weight, verifier_weight)
}

fn reputation_factor(score_micros: u32) -> MicroIPN {
    let clamped = score_micros
        .clamp(REPUTATION_SCORE_MIN, REPUTATION_SCORE_MAX) as u128;
    (clamped * 100) / (REPUTATION_SCORE_SCALE as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_cap_application() {
        let params = EconomicsParams {
            fee_cap_numer: 1,
            fee_cap_denom: 10, // 10% cap
            ..Default::default()
        };

        let emission = 1_000_000;
        let fees = 50_000; // 5% of emission
        let (capped_fees, cap_applied) = apply_fee_cap(fees, emission, &params);

        assert_eq!(capped_fees, 50_000); // No cap applied
        assert_eq!(cap_applied, 0);

        let fees = 200_000; // 20% of emission
        let (capped_fees, cap_applied) = apply_fee_cap(fees, emission, &params);

        assert_eq!(capped_fees, 100_000); // Capped to 10%
        assert_eq!(cap_applied, 100_000);
    }

    #[test]
    fn fee_cap_disabled_when_denom_zero() {
        let params = EconomicsParams {
            fee_cap_numer: 0,
            fee_cap_denom: 0,
            ..Default::default()
        };

        let (allowed, capped) = apply_fee_cap(5_000, 10_000, &params);
        assert_eq!(allowed, 0);
        assert_eq!(capped, 5_000);
    }

    #[test]
    fn test_reward_distribution() {
        let params = EconomicsParams::default();
        let participants = vec![
            Participation {
                validator_id: [1u8; 32],
                role: Role::Proposer,
                blocks_proposed: 1,
                blocks_verified: 0,
                reputation_score_micros: REPUTATION_SCORE_SCALE,
                stake_weight: 1000,
            },
            Participation {
                validator_id: [2u8; 32],
                role: Role::Verifier,
                blocks_proposed: 0,
                blocks_verified: 2,
                reputation_score_micros: REPUTATION_SCORE_SCALE,
                stake_weight: 1000,
            },
        ];

        let (payouts, emission, _) =
            distribute_round(1_000_000, 0, &participants, &params).unwrap();

        assert_eq!(emission, 1_000_000);
        assert_eq!(payouts.len(), 2);
        assert!(payouts.contains_key(&[1u8; 32]));
        assert!(payouts.contains_key(&[2u8; 32]));
    }

    #[test]
    fn test_empty_participants() {
        let params = EconomicsParams::default();
        let participants = vec![];

        let (payouts, returned_emission, _) =
            distribute_round(1_000_000, 0, &participants, &params).unwrap();

        assert_eq!(payouts.len(), 0);
        // The emission passed in should be returned unchanged
        assert_eq!(returned_emission, 1_000_000);
    }

    #[test]
    fn test_contribution_multiplier() {
        let mut participant = Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 0,
            blocks_verified: 0,
            reputation_score_micros: REPUTATION_SCORE_SCALE,
            stake_weight: 1000,
        };

        // No contribution
        assert_eq!(calculate_contribution_multiplier(&participant), 0);

        // Minimal contribution
        participant.blocks_proposed = 1;
        assert_eq!(calculate_contribution_multiplier(&participant), 800);

        // Normal contribution
        participant.blocks_proposed = 3;
        assert_eq!(calculate_contribution_multiplier(&participant), 1_000);

        // High contribution
        participant.blocks_proposed = 8;
        assert_eq!(calculate_contribution_multiplier(&participant), 1_200);

        // Very high contribution
        participant.blocks_proposed = 15;
        assert_eq!(calculate_contribution_multiplier(&participant), 1_500);
    }

    #[test]
    fn test_participation_validation() {
        let valid_participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score_micros: REPUTATION_SCORE_SCALE,
            stake_weight: 1000,
        }];

        assert!(validate_participation_set(&valid_participants).is_ok());

        let invalid_participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score_micros: REPUTATION_SCORE_SCALE,
            stake_weight: 0, // Invalid: zero stake
        }];

        assert!(validate_participation_set(&invalid_participants).is_err());
    }

    #[test]
    fn distribution_errors_when_no_weights() {
        let params = EconomicsParams::default();
        let participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 0,
            blocks_verified: 0,
            reputation_score_micros: REPUTATION_SCORE_SCALE,
            stake_weight: 1,
        }];

        let err = distribute_round(1_000, 500, &participants, &params).unwrap_err();
        assert!(matches!(err, EconomicsError::NoParticipants));
    }
}
