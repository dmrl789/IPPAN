#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_halving() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            ..Default::default()
        };

        // First halving epoch (rounds 1–1000)
        assert_eq!(emission_for_round_capped(1, 0, &params).unwrap(), 1_000_000);
        assert_eq!(emission_for_round_capped(500, 0, &params).unwrap(), 1_000_000);
        assert_eq!(emission_for_round_capped(1000, 0, &params).unwrap(), 1_000_000);

        // Second halving epoch (rounds 1001–2000)
        assert_eq!(emission_for_round_capped(1001, 0, &params).unwrap(), 500_000);
        assert_eq!(emission_for_round_capped(1500, 0, &params).unwrap(), 500_000);
        assert_eq!(emission_for_round_capped(2000, 0, &params).unwrap(), 500_000);

        // Third halving epoch (rounds 2001–3000)
        assert_eq!(emission_for_round_capped(2001, 0, &params).unwrap(), 250_000);
        assert_eq!(emission_for_round_capped(3000, 0, &params).unwrap(), 250_000);
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            max_supply_micro: 5_000_000, // Very low cap for testing
            ..Default::default()
        };

        // Should be capped at remaining supply
        let emission = emission_for_round_capped(1, 4_000_000, &params).unwrap();
        assert_eq!(emission, 1_000_000); // Normal emission

        let emission = emission_for_round_capped(2, 4_500_000, &params).unwrap();
        assert_eq!(emission, 500_000); // Capped to remaining supply

        let emission = emission_for_round_capped(3, 5_000_000, &params).unwrap();
        assert_eq!(emission, 0); // Cap reached
    }

    #[test]
    fn test_zero_round() {
        let params = EconomicsParams::default();
        assert_eq!(emission_for_round_capped(0, 0, &params).unwrap(), 0);
    }

    #[test]
    fn test_projected_supply() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            max_supply_micro: 10_000_000_000, // Higher cap for this test
            ..Default::default()
        };

        // First 1000 rounds: 1_000_000 micro-IPN per round
        let supply_1000 = project_total_supply(1000, &params);
        assert_eq!(supply_1000, 1_000_000 * 1000);

        // Next 1000 rounds: 500_000 micro-IPN per round
        let supply_2000 = project_total_supply(2000, &params);
        assert_eq!(supply_2000, 1_000_000 * 1000 + 500_000 * 1000);
    }

    #[test]
    fn test_emission_details() {
        let params = EconomicsParams::default();
        let details = get_emission_details(1000, 0, &params).unwrap();

        assert_eq!(details.round, 1000);
        assert!(details.emission_micro > 0);
        assert_eq!(details.total_issued_micro, details.emission_micro);
        assert!(details.remaining_cap_micro > 0);
        assert_eq!(details.halving_epoch, 0);
    }
}
