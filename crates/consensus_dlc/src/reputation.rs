use std::collections::HashMap;

#[derive(Default)]
pub struct ReputationDB {
    pub scores: HashMap<String, f64>,
}

impl ReputationDB {
    pub fn update(&mut self, node: &str, delta: f64) {
        *self.scores.entry(node.to_string()).or_insert(0.0) += delta;
    }

    pub fn score(&self, node: &str) -> f64 {
        *self.scores.get(node).unwrap_or(&0.0)
    }
}
