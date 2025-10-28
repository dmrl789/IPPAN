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
