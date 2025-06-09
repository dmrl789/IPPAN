use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionKind {
    RegisterHandle,
    UpdateHandle,
    RenewHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub handle: String,
    pub pubkey: Option<String>,
    pub kind: TransactionKind,
}

impl Transaction {
    pub fn new(handle: &str, pubkey: Option<String>, kind: TransactionKind) -> Self {
        Self {
            handle: handle.into(),
            pubkey,
            kind,
        }
    }
}
