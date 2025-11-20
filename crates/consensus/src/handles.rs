use ippan_l1_handle_anchors::{HandleAnchorError, HandleOwnershipAnchor, L1HandleAnchorStorage};
use ippan_l2_handle_registry::{
    dht::{HandleDhtRecord, HandleDhtService},
    Handle, HandleRegistration, HandleRegistryError, L2HandleRegistry, PublicKey,
};
use ippan_types::{HandleOperation, HandleRegisterOp, Transaction};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::runtime::Builder;
use tracing::warn;

/// Deterministic pipeline that applies handle transactions during round finalization.
pub struct HandlePipeline {
    registry: Arc<L2HandleRegistry>,
    anchors: Arc<L1HandleAnchorStorage>,
    handle_dht: Option<Arc<dyn HandleDhtService>>,
}

impl HandlePipeline {
    pub fn new(registry: Arc<L2HandleRegistry>, anchors: Arc<L1HandleAnchorStorage>) -> Self {
        Self::with_dht(registry, anchors, None)
    }

    pub fn with_dht(
        registry: Arc<L2HandleRegistry>,
        anchors: Arc<L1HandleAnchorStorage>,
        handle_dht: Option<Arc<dyn HandleDhtService>>,
    ) -> Self {
        Self {
            registry,
            anchors,
            handle_dht,
        }
    }

    pub fn registry(&self) -> Arc<L2HandleRegistry> {
        self.registry.clone()
    }

    pub fn anchors(&self) -> Arc<L1HandleAnchorStorage> {
        self.anchors.clone()
    }

    pub fn apply(
        &self,
        tx: &Transaction,
        block_height: u64,
        round: u64,
    ) -> Result<(), HandleApplyError> {
        let op = tx
            .handle_operation()
            .ok_or(HandleApplyError::MissingOperation)?;
        match op {
            HandleOperation::Register(data) => {
                self.apply_registration(tx, data, block_height, round)
            }
        }
    }

    fn apply_registration(
        &self,
        tx: &Transaction,
        op: &HandleRegisterOp,
        block_height: u64,
        round: u64,
    ) -> Result<(), HandleApplyError> {
        if op.owner != tx.from {
            return Err(HandleApplyError::OwnerMismatch);
        }

        if op.signature.len() != 64 {
            return Err(HandleApplyError::InvalidSignatureLength);
        }

        let handle = Handle::new(op.handle.clone());
        if !handle.is_valid() {
            return Err(HandleApplyError::InvalidHandle(op.handle.clone()));
        }

        if let Some(exp) = op.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if exp <= now {
                return Err(HandleApplyError::Expired {
                    expires_at: exp,
                    now,
                });
            }
        }

        let metadata: HashMap<String, String> = op
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let registration = HandleRegistration {
            handle: handle.clone(),
            owner: PublicKey::new(op.owner),
            signature: op.signature.clone(),
            metadata,
            expires_at: op.expires_at,
        };

        self.registry
            .register(registration)
            .map_err(HandleApplyError::Registry)?;

        let anchor = HandleOwnershipAnchor::new(
            handle.as_str(),
            op.owner,
            self.compute_l2_location(handle.as_str(), &op.owner),
            block_height,
            round,
            op.signature.clone(),
        );

        self.anchors
            .store_anchor(anchor)
            .map_err(HandleApplyError::Anchor)?;

        let dht_record = HandleDhtRecord::new(handle, PublicKey::new(op.owner), op.expires_at);
        self.publish_to_dht(dht_record);

        Ok(())
    }

    fn compute_l2_location(&self, handle: &str, owner: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(handle.as_bytes());
        hasher.update(owner);
        hasher.finalize().into()
    }

    fn publish_to_dht(&self, record: HandleDhtRecord) {
        let Some(service) = &self.handle_dht else {
            return;
        };

        let service = service.clone();
        let handle_label = record.handle.as_str().to_string();
        let future_label = handle_label.clone();
        let future = async move {
            if let Err(error) = service.publish_handle(&record).await {
                warn!(handle = future_label, error = %error, "failed to publish handle to DHT");
            }
        };

        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(future);
        } else if let Ok(rt) = Builder::new_current_thread().enable_all().build() {
            rt.block_on(future);
        } else {
            warn!(
                handle = handle_label,
                "failed to publish handle to DHT because no runtime was available"
            );
        }
    }
}

#[derive(Debug, Error)]
pub enum HandleApplyError {
    #[error("transaction missing handle operation payload")]
    MissingOperation,
    #[error("handle owner does not match transaction sender")]
    OwnerMismatch,
    #[error("handle string '{0}' is invalid")]
    InvalidHandle(String),
    #[error("handle registration expired (expires_at={expires_at}, now={now})")]
    Expired { expires_at: u64, now: u64 },
    #[error("handle signature must be 64 bytes")]
    InvalidSignatureLength,
    #[error("handle registry error: {0}")]
    Registry(#[from] HandleRegistryError),
    #[error("handle anchor error: {0}")]
    Anchor(#[from] HandleAnchorError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use ippan_l2_handle_registry::dht::StubHandleDhtService;
    use ippan_types::{Amount, HandleRegisterOp, Transaction};
    use std::collections::BTreeMap;
    use tokio::task::yield_now;

    fn make_transaction(
        handle: &str,
        signing: &SigningKey,
        expires_at: Option<u64>,
    ) -> Transaction {
        let owner = signing.verifying_key().to_bytes();
        let mut tx = Transaction::new(owner, [0u8; 32], Amount::zero(), 1);
        let op = HandleRegisterOp {
            handle: handle.to_string(),
            owner,
            metadata: BTreeMap::new(),
            expires_at,
            signature: sign_registration(signing, handle, owner, expires_at),
        };
        tx.set_handle_operation(HandleOperation::Register(op));
        tx.sign(&signing.to_bytes()).unwrap();
        tx
    }

    fn sign_registration(
        signing: &SigningKey,
        handle: &str,
        owner: [u8; 32],
        expires_at: Option<u64>,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"IPPAN_HANDLE_REGISTRATION");
        payload.extend_from_slice(handle.as_bytes());
        payload.extend_from_slice(&owner);
        if let Some(exp) = expires_at {
            payload.extend_from_slice(&exp.to_le_bytes());
        }
        let digest = Sha256::digest(&payload);
        signing.sign(&digest).to_bytes().to_vec()
    }

    #[tokio::test]
    async fn applies_registration_and_anchor() {
        let registry = Arc::new(L2HandleRegistry::new());
        let anchors = Arc::new(L1HandleAnchorStorage::new());
        let pipeline = HandlePipeline::new(registry.clone(), anchors.clone());

        let signing = SigningKey::from_bytes(&[42u8; 32]);
        let tx = make_transaction(
            "@demo.ipn",
            &signing,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 60,
            ),
        );

        pipeline
            .apply(&tx, 10, 10)
            .expect("handle registration succeeds");

        let resolved_owner = registry
            .resolve(&Handle::new("@demo.ipn"))
            .expect("handle stored");
        assert_eq!(
            resolved_owner.as_bytes(),
            &signing.verifying_key().to_bytes()
        );
        assert!(anchors.get_anchor_by_handle("@demo.ipn").is_ok());
    }

    #[tokio::test]
    async fn rejects_duplicate_handles() {
        let registry = Arc::new(L2HandleRegistry::new());
        let anchors = Arc::new(L1HandleAnchorStorage::new());
        let pipeline = HandlePipeline::new(registry.clone(), anchors);
        let signing = SigningKey::from_bytes(&[11u8; 32]);
        let tx = make_transaction(
            "@taken.ipn",
            &signing,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 60,
            ),
        );
        pipeline.apply(&tx, 1, 1).unwrap();

        let duplicate = make_transaction(
            "@taken.ipn",
            &signing,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 120,
            ),
        );
        let err = pipeline.apply(&duplicate, 2, 2).unwrap_err();
        assert!(matches!(err, HandleApplyError::Registry(_)));
    }

    #[tokio::test]
    async fn rejects_invalid_expiry() {
        let registry = Arc::new(L2HandleRegistry::new());
        let anchors = Arc::new(L1HandleAnchorStorage::new());
        let pipeline = HandlePipeline::new(registry, anchors);
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let tx = make_transaction(
            "@expired.ipn",
            &signing,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .saturating_sub(5),
            ),
        );
        let err = pipeline.apply(&tx, 1, 1).unwrap_err();
        assert!(matches!(err, HandleApplyError::Expired { .. }));
    }

    #[tokio::test]
    async fn publishes_handle_records_into_dht() {
        let registry = Arc::new(L2HandleRegistry::new());
        let anchors = Arc::new(L1HandleAnchorStorage::new());
        let dht = Arc::new(StubHandleDhtService::new());
        let pipeline = HandlePipeline::with_dht(registry.clone(), anchors, Some(dht.clone()));

        let signing = SigningKey::from_bytes(&[33u8; 32]);
        let tx = make_transaction(
            "@dht-demo.ipn",
            &signing,
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 30,
            ),
        );

        pipeline.apply(&tx, 5, 5).expect("registration succeeds");
        yield_now().await;

        let record = dht
            .get(&Handle::new("@dht-demo.ipn"))
            .expect("record published");
        assert_eq!(record.owner.as_bytes(), &signing.verifying_key().to_bytes());
    }
}
