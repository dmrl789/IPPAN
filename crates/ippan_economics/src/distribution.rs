#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_participation(
        validator_id: &str,
        role: ValidatorRole,
        blocks: u32,
    ) -> ValidatorParticipation {
        ValidatorParticipation {
            validator_id: ValidatorId::new(validator_id),
            role,
            blocks_contributed: blocks,
            uptime_score: Decimal::ONE,
        }
    }

    #[test]
    fn test_reward_distribution() {
        let rewards = RoundRewards::new(EmissionParams::default());

        let participations = vec![
            create_test_participation("validator1", ValidatorRole::Proposer, 10),
            create_test_participation("validator2", ValidatorRole::Verifier, 5),
        ];

        let distribution = rewards
            .distribute_round_rewards(1, 10_000, participations, 1_000)
            .unwrap();

        assert_eq!(distribution.round_index, 1);
        assert_eq!(distribution.total_reward, 11_000); // 10_000 round reward + 1_000 fees
        assert_eq!(distribution.validator_rewards.len(), 2);

        // Proposer should get more reward than verifier
        let proposer_reward = distribution
            .validator_rewards
            .get(&ValidatorId::new("validator1"))
            .unwrap();
        let verifier_reward = distribution
            .validator_rewards
            .get(&ValidatorId::new("validator2"))
            .unwrap();

        assert!(proposer_reward.total_reward > verifier_reward.total_reward);
    }

    #[test]
    fn test_fee_cap() {
        let mut params = EmissionParams::default();
        params.fee_cap_fraction = Decimal::new(1, 1); // 10%

        let rewards = RoundRewards::new(params);
        let capped_fees = rewards.apply_fee_cap(5_000, 10_000);

        assert_eq!(capped_fees, 1_000); // 10% of 10,000
    }

    #[test]
    fn test_empty_participation() {
        let rewards = RoundRewards::new(EmissionParams::default());
        let result = rewards.distribute_round_rewards(1, 10_000, vec![], 0);

        assert!(matches!(result, Err(DistributionError::NoValidators(1))));
    }
}
