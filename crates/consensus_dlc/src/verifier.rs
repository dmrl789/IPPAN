use crate::{dgbdt::{FairnessModel, ValidatorMetrics}, dag::Block};
use blake3::Hasher;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

#[derive(Clone, Debug)]
pub struct VerifierSet {
    pub primary: String,
    pub shadows: Vec<String>,
}

impl VerifierSet {
    pub fn select(model: &FairnessModel, seed: impl Into<String>) -> Self {
        let candidates = vec![
            ("nodeA".to_string(), ValidatorMetrics { uptime: 0.99, latency: 0.10, honesty: 1.0 }),
            ("nodeB".to_string(), ValidatorMetrics { uptime: 0.97, latency: 0.20, honesty: 0.95 }),
            ("nodeC".to_string(), ValidatorMetrics { uptime: 0.95, latency: 0.15, honesty: 0.98 }),
        ];

        let mut scored: Vec<(String, f64)> = candidates
            .into_iter()
            .map(|(id, metrics)| (id, model.score(&metrics)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut hasher = Hasher::new();
        let seed_string = seed.into();
        hasher.update(seed_string.as_bytes());
        let mut seed_bytes = [0u8; 32];
        let hash = hasher.finalize();
        seed_bytes.copy_from_slice(hash.as_bytes());
        let mut rng = StdRng::from_seed(seed_bytes);

        let mut validators: Vec<String> = scored.into_iter().map(|(id, _)| id).collect();
        validators.shuffle(&mut rng);

        let primary = validators.first().cloned().unwrap_or_default();
        let shadows = validators.into_iter().skip(1).collect();
        Self { primary, shadows }
    }

    pub fn collect_blocks(&self, pool: Vec<Block>) -> Vec<Block> {
        pool
    }

    pub fn validate(&self, _block: &Block) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedBlock(pub Block);
