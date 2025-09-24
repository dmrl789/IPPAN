use crate::hashtimer::{random_nonce, HashTimer, IppanTimeMicros};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Supported proof systems for L2 state commitments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum L2ProofType {
    ZkGroth16,
    Optimistic,
    External,
    Inline,
}

impl Default for L2ProofType {
    fn default() -> Self {
        Self::ZkGroth16
    }
}

impl L2ProofType {
    /// String representation used for hashing and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ZkGroth16 => "zk-groth16",
            Self::Optimistic => "optimistic",
            Self::External => "external",
            Self::Inline => "inline",
        }
    }
}

/// Lifecycle state of an L2 exit request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum L2ExitStatus {
    Pending,
    ChallengeWindow,
    Finalized,
    Rejected,
}

impl Default for L2ExitStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Anchored state commitment from an L2 rollup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2StateCommit {
    /// Deterministic commitment identifier.
    pub id: [u8; 32],
    /// Rollup identifier.
    pub l2_id: String,
    /// Sequential epoch identifier inside the rollup.
    pub epoch: u64,
    /// State root commitment (hex encoded).
    pub state_root: String,
    /// Optional data availability hash.
    pub da_hash: Option<String>,
    /// Proof system used when generating the commitment.
    pub proof_type: L2ProofType,
    /// Proof payload (hex/base64 encoded, depending on proof type).
    pub proof: Option<String>,
    /// Optional inline data payload (used when DA mode is inline).
    pub inline_data: Option<String>,
    /// Temporal proof binding the commitment to IPPAN time.
    pub hashtimer: HashTimer,
    /// Creation timestamp in IPPAN time microseconds.
    pub timestamp: IppanTimeMicros,
}

impl L2StateCommit {
    /// Create a new commitment using the current IPPAN time.
    pub fn new(
        l2_id: impl Into<String>,
        epoch: u64,
        state_root: impl Into<String>,
        da_hash: Option<String>,
        proof_type: L2ProofType,
        proof: Option<String>,
        inline_data: Option<String>,
        node_id: &[u8],
    ) -> Self {
        Self::with_timestamp(
            l2_id,
            epoch,
            state_root,
            da_hash,
            proof_type,
            proof,
            inline_data,
            IppanTimeMicros::now(),
            node_id,
        )
    }

    /// Create a commitment with an explicit timestamp. Primarily used for validation before
    /// persistence, allowing the caller to enforce minimum epoch spacing.
    pub fn with_timestamp(
        l2_id: impl Into<String>,
        epoch: u64,
        state_root: impl Into<String>,
        da_hash: Option<String>,
        proof_type: L2ProofType,
        proof: Option<String>,
        inline_data: Option<String>,
        timestamp: IppanTimeMicros,
        node_id: &[u8],
    ) -> Self {
        let l2_id = l2_id.into();
        let state_root = state_root.into();
        let payload = Self::payload(
            &l2_id,
            epoch,
            &state_root,
            da_hash.as_deref(),
            &proof_type,
            proof.as_deref(),
            inline_data.as_deref(),
        );
        let nonce = random_nonce();
        let hashtimer = HashTimer::derive("l2_commit", timestamp, b"l2", &payload, &nonce, node_id);

        let mut commit = Self {
            id: [0u8; 32],
            l2_id,
            epoch,
            state_root,
            da_hash,
            proof_type,
            proof,
            inline_data,
            hashtimer,
            timestamp,
        };
        commit.refresh_id();
        commit
    }

    /// Recompute the commitment identifier.
    pub fn refresh_id(&mut self) {
        self.id = self.compute_hash();
    }

    fn payload(
        l2_id: &str,
        epoch: u64,
        state_root: &str,
        da_hash: Option<&str>,
        proof_type: &L2ProofType,
        proof: Option<&str>,
        inline_data: Option<&str>,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(l2_id.as_bytes());
        payload.extend_from_slice(&epoch.to_be_bytes());
        payload.extend_from_slice(state_root.as_bytes());
        if let Some(hash) = da_hash {
            payload.extend_from_slice(hash.as_bytes());
        }
        payload.extend_from_slice(proof_type.as_str().as_bytes());
        if let Some(proof) = proof {
            payload.extend_from_slice(proof.as_bytes());
        }
        if let Some(data) = inline_data {
            payload.extend_from_slice(data.as_bytes());
        }
        payload
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(self.l2_id.as_bytes());
        hasher.update(&self.epoch.to_be_bytes());
        hasher.update(self.state_root.as_bytes());
        if let Some(ref hash) = self.da_hash {
            hasher.update(hash.as_bytes());
        }
        hasher.update(self.proof_type.as_str().as_bytes());
        if let Some(ref proof) = self.proof {
            hasher.update(proof.as_bytes());
        }
        if let Some(ref data) = self.inline_data {
            hasher.update(data.as_bytes());
        }
        hasher.update(self.hashtimer.to_hex().as_bytes());
        hasher.update(&self.timestamp.0.to_be_bytes());

        let hash = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash.as_bytes()[0..32]);
        id
    }
}

/// Exit request from an L2 back to IPPAN L1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Exit {
    /// Deterministic exit identifier.
    pub id: [u8; 32],
    /// Rollup identifier.
    pub l2_id: String,
    /// Rollup epoch the exit proof references.
    pub epoch: u64,
    /// Account initiating the exit (string encoded address).
    pub account: String,
    /// Amount being exited in micro-IPPAN (1e-6 precision).
    pub amount: u128,
    /// Account nonce on the rollup.
    pub nonce: u64,
    /// Merkle proof of inclusion or fraud proof reference.
    pub proof_of_inclusion: String,
    /// Temporal binding for ordering and verification.
    pub hashtimer: HashTimer,
    /// Current exit status.
    pub status: L2ExitStatus,
    /// Submission timestamp.
    pub submitted_at: IppanTimeMicros,
    /// Finalization timestamp, when applicable.
    pub finalized_at: Option<IppanTimeMicros>,
    /// Optional rejection reason recorded by the bridge.
    pub rejection_reason: Option<String>,
}

impl L2Exit {
    /// Create a new exit entry using the current IPPAN time.
    pub fn new(
        l2_id: impl Into<String>,
        epoch: u64,
        account: impl Into<String>,
        amount: u128,
        nonce: u64,
        proof_of_inclusion: impl Into<String>,
        node_id: &[u8],
    ) -> Self {
        Self::with_timestamp(
            l2_id,
            epoch,
            account,
            amount,
            nonce,
            proof_of_inclusion,
            IppanTimeMicros::now(),
            node_id,
        )
    }

    /// Create a new exit entry with an explicit timestamp.
    pub fn with_timestamp(
        l2_id: impl Into<String>,
        epoch: u64,
        account: impl Into<String>,
        amount: u128,
        nonce: u64,
        proof_of_inclusion: impl Into<String>,
        timestamp: IppanTimeMicros,
        node_id: &[u8],
    ) -> Self {
        let l2_id = l2_id.into();
        let account = account.into();
        let proof_of_inclusion = proof_of_inclusion.into();
        let payload = Self::payload(&l2_id, epoch, &account, amount, nonce, &proof_of_inclusion);
        let nonce_bytes = random_nonce();
        let hashtimer =
            HashTimer::derive("l2_exit", timestamp, b"l2", &payload, &nonce_bytes, node_id);

        let mut exit = Self {
            id: [0u8; 32],
            l2_id,
            epoch,
            account,
            amount,
            nonce,
            proof_of_inclusion,
            hashtimer,
            status: L2ExitStatus::Pending,
            submitted_at: timestamp,
            finalized_at: None,
            rejection_reason: None,
        };
        exit.refresh_id();
        exit
    }

    /// Update the exit identifier from the current contents.
    pub fn refresh_id(&mut self) {
        self.id = self.compute_hash();
    }

    /// Helper to compute the end of the challenge window based on the provided bridge policy.
    pub fn challenge_window_deadline(&self, challenge_window_ms: u64) -> IppanTimeMicros {
        let additional = challenge_window_ms.saturating_mul(1_000);
        IppanTimeMicros(self.submitted_at.0.saturating_add(additional))
    }

    fn payload(
        l2_id: &str,
        epoch: u64,
        account: &str,
        amount: u128,
        nonce: u64,
        proof_of_inclusion: &str,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(l2_id.as_bytes());
        payload.extend_from_slice(&epoch.to_be_bytes());
        payload.extend_from_slice(account.as_bytes());
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(&nonce.to_be_bytes());
        payload.extend_from_slice(proof_of_inclusion.as_bytes());
        payload
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(self.l2_id.as_bytes());
        hasher.update(&self.epoch.to_be_bytes());
        hasher.update(self.account.as_bytes());
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(self.proof_of_inclusion.as_bytes());
        hasher.update(self.hashtimer.to_hex().as_bytes());
        hasher.update(&self.submitted_at.0.to_be_bytes());

        let hash = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash.as_bytes()[0..32]);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_determinism() {
        let node_id = b"node";
        let timestamp = IppanTimeMicros(42);
        let mut commit = L2StateCommit::with_timestamp(
            "rollup-1",
            7,
            "0xabc",
            Some("0xdef".to_string()),
            L2ProofType::ZkGroth16,
            Some("0xproof".to_string()),
            None,
            timestamp,
            node_id,
        );
        let original_id = commit.id;
        commit.refresh_id();
        assert_eq!(commit.id, original_id);
        assert_eq!(commit.hashtimer.to_hex().len(), 64);
    }

    #[test]
    fn test_exit_challenge_deadline() {
        let node_id = b"node";
        let timestamp = IppanTimeMicros(1_000_000);
        let exit = L2Exit::with_timestamp(
            "rollup-1",
            10,
            "0xaccount",
            500,
            3,
            "proof",
            timestamp,
            node_id,
        );

        assert_eq!(exit.status, L2ExitStatus::Pending);
        assert_eq!(
            exit.challenge_window_deadline(1_500),
            IppanTimeMicros(2_500_000)
        );
    }
}
