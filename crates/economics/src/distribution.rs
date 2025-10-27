//! DAG-Fair reward distribution with fee capping and recycling

use crate::types::{EconomicsParams, MicroIPN, Participation, ParticipationSet, Payouts, Role};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Distribute rewards for a round with fee capping and fair allocation
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    participants: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN)> {
    if participants.is_empty() {
        return Ok((HashMap::new(), 0, 0));
    }

    // Apply fee cap
    let (capped_fees_micro, fee_cap_applied_micro) =
        apply_fee_cap(fees_micro, emission_micro, params);

    // Calculate total reward pool (emission + recycled fees)
    let fee_recycling_micro = (capped_fees_micro * params.fee_recycling_bps as u128) / 10_000;
    let total_reward_micro = emission_micro.saturating_add(fee_recycling_micro);

    // Distribute rewards fairly among participants
    let payouts = distribute_rewards_fairly(total_reward_micro, participants, params)?;

    info!(
        target: "distribution",
        "Round distribution: {} emission + {} recycled fees = {} total, {} payouts",
        emission_micro,
        fee_recycling_micro,
        total_reward_micro,
        payouts.len()
    );

    Ok((payouts, emission_micro, fee_cap_applied_micro))
}

/// Apply fee cap to prevent excessive fee collection
fn apply_fee_cap(
    fees_micro: MicroIPN,
    emission_micro: MicroIPN,
    params: &EconomicsParams,
) -> (MicroIPN, MicroIPN) {
    let max_fees_micro =
        (emission_micro * params.fee_cap_numer as u128) / params.fee_cap_denom as u128;

    if fees_micro <= max_fees_micro {
        (fees_micro, 0)
    } else {
        let capped_fees = max_fees_micro;
        let cap_applied = fees_micro - max_fees_micro;

        warn!(
            target: "distribution",
            "Fee cap applied: {} -> {} micro-IPN (capped by {} micro-IPN)",
            fees_micro,
            capped_fees,
            cap_applied
        );

        (capped_fees, cap_applied)
    }
}

/// Distribute rewards fairly among participants based on role and contribution
fn distribute_rewards_fairly(
    total_reward_micro: MicroIPN,
    participants: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<Payouts> {
    let mut payouts = HashMap::new();

    if total_reward_micro == 0 {
        return Ok(payouts);
    }

    // Calculate total weight for all participants
    let total_weight = calculate_total_weight(participants, params);

    if total_weight == 0 {
        return Err(anyhow!("No valid participants with weight"));
    }

    // Distribute rewards proportionally
    for participant in participants {
        let weight = calculate_participant_weight(participant, params);
        if weight == 0 {
            continue;
        }

        let reward = (total_reward_micro * weight) / total_weight;
        if reward > 0 {
            payouts.insert(participant.validator_id, reward);

            debug!(
                target: "distribution",
                "Validator {:?}: {} micro-IPN (weight: {})",
                participant.validator_id,
                reward,
                weight
            );
        }
    }

    Ok(payouts)
}

/// Calculate total weight for all participants
fn calculate_total_weight(participants: &ParticipationSet, params: &EconomicsParams) -> u128 {
    participants
        .iter()
        .map(|p| calculate_participant_weight(p, params))
        .sum()
}

/// Calculate weight for a single participant based on role and contribution
fn calculate_participant_weight(participant: &Participation, params: &EconomicsParams) -> u128 {
    let mut weight = participant.stake_weight;

    // Apply role-based multipliers
    match participant.role {
        Role::Proposer => {
            weight = (weight * params.proposer_weight_bps as u128) / 10_000;
        }
        Role::Verifier => {
            weight = (weight * params.verifier_weight_bps as u128) / 10_000;
        }
        Role::Both => {
            // For participants who are both proposer and verifier, use the higher weight
            let proposer_weight = (weight * params.proposer_weight_bps as u128) / 10_000;
            let verifier_weight = (weight * params.verifier_weight_bps as u128) / 10_000;
            weight = proposer_weight.max(verifier_weight);
        }
    }

    // Apply reputation multiplier (0.5 to 2.0 range)
    let reputation_multiplier = (participant.reputation_score * 1000.0) as u128;
    let reputation_multiplier = reputation_multiplier.clamp(500, 2000); // 0.5x to 2.0x
    weight = (weight * reputation_multiplier) / 1000;

    // Apply contribution multiplier based on blocks proposed/verified
    let contribution_multiplier = calculate_contribution_multiplier(participant);
    weight = (weight * contribution_multiplier) / 1000;

    weight
}

/// Calculate contribution multiplier based on blocks proposed/verified
fn calculate_contribution_multiplier(participant: &Participation) -> u128 {
    let total_blocks = participant.blocks_proposed + participant.blocks_verified;

    match total_blocks {
        0 => 0,         // No contribution
        1 => 800,       // 0.8x for minimal contribution
        2..=5 => 1000,  // 1.0x for normal contribution
        6..=10 => 1200, // 1.2x for high contribution
        _ => 1500,      // 1.5x for very high contribution
    }
}

/// Calculate total rewards for a validator across all rounds
pub fn calculate_validator_total_rewards(
    validator_id: &[u8; 32],
    all_payouts: &[Payouts],
) -> MicroIPN {
    all_payouts
        .iter()
        .flat_map(|payouts| payouts.get(validator_id))
        .copied()
        .sum()
}

/// Validate participation set for distribution
pub fn validate_participation_set(participants: &ParticipationSet) -> Result<()> {
    if participants.is_empty() {
        return Err(anyhow!("Empty participation set"));
    }

    for participant in participants {
        if participant.stake_weight == 0 {
            return Err(anyhow!("Participant with zero stake weight"));
        }

        if participant.reputation_score < 0.0 || participant.reputation_score > 2.0 {
            return Err(anyhow!(
                "Invalid reputation score: {}",
                participant.reputation_score
            ));
        }
    }

    Ok(())
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
    fn test_reward_distribution() {
        let params = EconomicsParams::default();
        let participants = vec![
            Participation {
                validator_id: [1u8; 32],
                role: Role::Proposer,
                blocks_proposed: 1,
                blocks_verified: 0,
                reputation_score: 1.0,
                stake_weight: 1000,
            },
            Participation {
                validator_id: [2u8; 32],
                role: Role::Verifier,
                blocks_proposed: 0,
                blocks_verified: 2,
                reputation_score: 1.0,
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

        let (payouts, emission, _) =
            distribute_round(1_000_000, 0, &participants, &params).unwrap();

        assert_eq!(payouts.len(), 0);
        assert_eq!(emission, 1_000_000);
    }

    #[test]
    fn test_contribution_multiplier() {
        let mut participant = Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 0,
            blocks_verified: 0,
            reputation_score: 1.0,
            stake_weight: 1000,
        };

        // No contribution
        assert_eq!(calculate_contribution_multiplier(&participant), 0);

        // Minimal contribution
        participant.blocks_proposed = 1;
        assert_eq!(calculate_contribution_multiplier(&participant), 800);

        // Normal contribution
        participant.blocks_proposed = 3;
        assert_eq!(calculate_contribution_multiplier(&participant), 1000);

        // High contribution
        participant.blocks_proposed = 8;
        assert_eq!(calculate_contribution_multiplier(&participant), 1200);

        // Very high contribution
        participant.blocks_proposed = 15;
        assert_eq!(calculate_contribution_multiplier(&participant), 1500);
    }

    #[test]
    fn test_participation_validation() {
        let valid_participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score: 1.0,
            stake_weight: 1000,
        }];

        assert!(validate_participation_set(&valid_participants).is_ok());

        let invalid_participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score: 1.0,
            stake_weight: 0, // Invalid: zero stake
        }];

        assert!(validate_participation_set(&invalid_participants).is_err());
    }
}
