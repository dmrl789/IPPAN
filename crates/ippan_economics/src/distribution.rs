//! Reward distribution logic for validators

use crate::types::{EconomicsParams, MicroIPN, Participation, ParticipationSet, Role, ValidatorId};
use std::collections::HashMap;

/// Distribute rewards for a round among participating validators
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    participation: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(HashMap<ValidatorId, MicroIPN>, MicroIPN, MicroIPN), &'static str> {
    if participation.is_empty() {
        return Ok((HashMap::new(), 0, 0));
    }

    let total_reward = emission_micro.saturating_add(fees_micro);
    
    // Find proposer and verifiers
    let mut proposer: Option<&ValidatorId> = None;
    let mut verifiers = Vec::new();
    
    for (vid, part) in participation {
        match part.role {
            Role::Proposer => proposer = Some(vid),
            Role::Verifier => verifiers.push(vid),
        }
    }

    let mut payouts = HashMap::new();
    
    // Distribute proposer reward
    if let Some(prop_id) = proposer {
        let proposer_reward = (total_reward * params.proposer_bps as u128) / 10_000;
        payouts.insert(prop_id.clone(), proposer_reward);
    }

    // Distribute verifier rewards
    if !verifiers.is_empty() {
        let verifier_pool = (total_reward * params.verifier_bps as u128) / 10_000;
        let per_verifier = verifier_pool / verifiers.len() as u128;
        
        for verifier_id in verifiers {
            payouts.insert(verifier_id.clone(), per_verifier);
        }
    }

    // Calculate actual amounts paid
    let emission_paid = emission_micro.min(total_reward);
    let fees_paid = fees_micro.min(total_reward.saturating_sub(emission_paid));

    Ok((payouts, emission_paid, fees_paid))
}

/// Calculate proportional reward based on participation
pub fn calculate_proportional_reward(
    total_reward: MicroIPN,
    participation: &Participation,
    total_participation: u32,
) -> MicroIPN {
    if total_participation == 0 {
        return 0;
    }
    
    (total_reward * participation.blocks as u128) / total_participation as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distribute_round() {
        let params = EconomicsParams::default();
        let mut participation = ParticipationSet::new();
        
        participation.insert(
            ValidatorId("alice".to_string()),
            Participation {
                role: Role::Proposer,
                blocks: 1,
            },
        );
        participation.insert(
            ValidatorId("bob".to_string()),
            Participation {
                role: Role::Verifier,
                blocks: 2,
            },
        );
        participation.insert(
            ValidatorId("carol".to_string()),
            Participation {
                role: Role::Verifier,
                blocks: 3,
            },
        );

        let (payouts, emission_paid, fees_paid) = distribute_round(
            1000,
            100,
            &participation,
            &params,
        ).unwrap();

        assert_eq!(emission_paid, 1000);
        assert_eq!(fees_paid, 100);
        assert_eq!(payouts.len(), 3);
        
        // Check that proposer gets 20% and verifiers split 80%
        let proposer_reward = payouts.get(&ValidatorId("alice".to_string())).unwrap();
        let verifier_reward = payouts.get(&ValidatorId("bob".to_string())).unwrap();
        
        assert_eq!(*proposer_reward, 220); // 20% of 1100
        assert_eq!(*verifier_reward, 440); // 40% of 80% of 1100
    }

    #[test]
    fn test_empty_participation() {
        let params = EconomicsParams::default();
        let participation = ParticipationSet::new();
        
        let (payouts, emission_paid, fees_paid) = distribute_round(
            1000,
            100,
            &participation,
            &params,
        ).unwrap();

        assert_eq!(payouts.len(), 0);
        assert_eq!(emission_paid, 0);
        assert_eq!(fees_paid, 0);
    }
}
