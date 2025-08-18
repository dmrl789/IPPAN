use serde::{Serialize, Deserialize};
use blake3::Hasher;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HashTimer {
    /// microsecond-level timestamp from IPPAN Time
    pub us: u64,
    /// round id / tie-breaker (bind to your rounds)
    pub round_id: u64,
}

pub fn blake3_hash(bytes: &[u8]) -> Hash {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finalize().into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricKV { pub key: String, pub value: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAsset {
    pub id: Hash,
    pub owner: Address,
    pub arch_id: u32,
    pub version: u32,
    pub weights_hash: Hash,
    pub size_bytes: u64,
    pub train_parent: Option<Hash>,
    pub train_config: Hash,
    pub license_id: u32,
    pub metrics: Vec<MetricKV>,
    pub provenance: Vec<Hash>,
    pub created_at: HashTimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetAsset {
    pub id: Hash,
    pub owner: Address,
    pub schema: String,
    pub shards: Vec<Hash>, // DHT keys or storage IDs
    pub license_id: u32,
    pub pii_flags: u32,
    pub consents: Vec<String>,
    pub quality_scores: Vec<MetricKV>,
    pub provenance: Vec<Hash>,
    pub created_at: HashTimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyMode { Open, TEE, Zk }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sla {
    pub max_latency_ms: u32,
    pub region: String,
    pub price_cap_ipn: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceJob {
    pub id: Hash,
    pub model_ref: Hash,
    pub input_commit: Hash, // commitment to the input payload
    pub sla: Sla,
    pub privacy: PrivacyMode,
    pub bid_window_ms: u16,
    pub max_price_ipn: u128,
    pub escrow_ipn: u128,
    pub created_at: HashTimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingTask {
    pub id: Hash,
    pub code_hash: Hash,
    pub model_base: Hash,
    pub dataset_refs: Vec<Hash>,
    pub objective: String,
    pub epochs: u32,
    pub checkpoint_freq: u32,
    pub sla: Sla,
    pub created_at: HashTimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation { pub blob: Vec<u8> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof { pub bytes: Vec<u8> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofReceipt {
    PoI {
        job_id: Hash,
        model_ref: Hash,
        output_commit: Hash,
        runtime_ms: u32,
        attest: Option<Attestation>,
        zk: Option<ZkProof>,
    },
    PoL {
        task_id: Hash,
        epoch: u32,
        ckpt_hash: Hash,
        grad_commit: Hash,
        samples_commit: Hash,
        attest: Option<Attestation>,
        zk: Option<ZkProof>,
    }
}
