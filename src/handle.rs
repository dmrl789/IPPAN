use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleRegistry {
    handles: HashMap<String, String>, // handle -> pubkey (as hex)
}

impl HandleRegistry {
    pub fn new() -> Self {
        Self { handles: HashMap::new() }
    }

    pub fn register(&mut self, handle: &str, pubkey_hex: &str) -> Result<(), String> {
        if self.handles.contains_key(handle) {
            return Err("Handle already taken".to_string());
        }
        self.handles.insert(handle.to_string(), pubkey_hex.to_string());
        Ok(())
    }

    pub fn get_pubkey(&self, handle: &str) -> Option<&String> {
        self.handles.get(handle)
    }

    pub fn is_registered(&self, handle: &str) -> bool {
        self.handles.contains_key(handle)
    }
}
