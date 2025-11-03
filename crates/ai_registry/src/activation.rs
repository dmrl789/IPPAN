use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model activation schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationSchedule {
    /// Models scheduled for activation by round
    scheduled_activations: HashMap<u64, Vec<String>>,
    /// Models scheduled for deactivation by round
    scheduled_deactivations: HashMap<u64, Vec<String>>,
}

/// Model activation manager
pub struct ActivationManager {
    schedule: ActivationSchedule,
    /// Current round
    current_round: u64,
}

impl ActivationManager {
    /// Create a new activation manager
    pub fn new() -> Self {
        Self {
            schedule: ActivationSchedule {
                scheduled_activations: HashMap::new(),
                scheduled_deactivations: HashMap::new(),
            },
            current_round: 0,
        }
    }

    /// Schedule a model for activation at a specific round
    pub fn schedule_activation(&mut self, model_id: String, activation_round: u64) {
        self.schedule
            .scheduled_activations
            .entry(activation_round)
            .or_default()
            .push(model_id);
    }

    /// Schedule a model for deactivation at a specific round
    pub fn schedule_deactivation(&mut self, model_id: String, deactivation_round: u64) {
        self.schedule
            .scheduled_deactivations
            .entry(deactivation_round)
            .or_default()
            .push(model_id);
    }

    /// Process activations and deactivations for the current round
    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        self.current_round = round;
        let mut activated_models = Vec::new();

        // Process activations
        if let Some(models) = self.schedule.scheduled_activations.remove(&round) {
            for model_id in models {
                activated_models.push(model_id);
            }
        }

        // Process deactivations
        if let Some(models) = self.schedule.scheduled_deactivations.remove(&round) {
            for model_id in models {
                // In a real implementation, you would deactivate the model here
                tracing::info!("Deactivating model {} at round {}", model_id, round);
            }
        }

        Ok(activated_models)
    }

    /// Get models scheduled for activation at a specific round
    pub fn get_scheduled_activations(&self, round: u64) -> Vec<&String> {
        self.schedule
            .scheduled_activations
            .get(&round)
            .map(|models| models.iter().collect())
            .unwrap_or_default()
    }

    /// Get models scheduled for deactivation at a specific round
    pub fn get_scheduled_deactivations(&self, round: u64) -> Vec<&String> {
        self.schedule
            .scheduled_deactivations
            .get(&round)
            .map(|models| models.iter().collect())
            .unwrap_or_default()
    }

    /// Get the current round
    pub fn current_round(&self) -> u64 {
        self.current_round
    }

    /// Advance to the next round
    pub fn advance_round(&mut self) -> Result<Vec<String>> {
        self.process_round(self.current_round + 1)
    }
}

impl Default for ActivationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_scheduling() {
        let mut manager = ActivationManager::new();

        manager.schedule_activation("model_1".to_string(), 100);
        manager.schedule_activation("model_2".to_string(), 100);
        manager.schedule_activation("model_3".to_string(), 200);

        let activations_100 = manager.get_scheduled_activations(100);
        assert_eq!(activations_100.len(), 2);
        assert!(activations_100.contains(&&"model_1".to_string()));
        assert!(activations_100.contains(&&"model_2".to_string()));

        let activations_200 = manager.get_scheduled_activations(200);
        assert_eq!(activations_200.len(), 1);
        assert!(activations_200.contains(&&"model_3".to_string()));
    }

    #[test]
    fn test_round_processing() {
        let mut manager = ActivationManager::new();

        manager.schedule_activation("model_1".to_string(), 100);
        manager.schedule_activation("model_2".to_string(), 100);
        manager.schedule_deactivation("old_model".to_string(), 100);

        let activated = manager.process_round(100).unwrap();
        assert_eq!(activated.len(), 2);
        assert!(activated.contains(&"model_1".to_string()));
        assert!(activated.contains(&"model_2".to_string()));

        // Models should be removed from schedule after processing
        let activations_100 = manager.get_scheduled_activations(100);
        assert_eq!(activations_100.len(), 0);
    }

    #[test]
    fn test_round_advancement() {
        let mut manager = ActivationManager::new();

        manager.schedule_activation("model_1".to_string(), 1);
        manager.schedule_activation("model_2".to_string(), 2);

        let activated_1 = manager.advance_round().unwrap();
        assert_eq!(activated_1.len(), 1);
        assert!(activated_1.contains(&"model_1".to_string()));
        assert_eq!(manager.current_round(), 1);

        let activated_2 = manager.advance_round().unwrap();
        assert_eq!(activated_2.len(), 1);
        assert!(activated_2.contains(&"model_2".to_string()));
        assert_eq!(manager.current_round(), 2);
    }
}
