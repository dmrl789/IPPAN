use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AddressBook {
    pub map: HashMap<String, String>, // handle -> address
}

impl AddressBook {
    pub fn new() -> Self {
        AddressBook { map: HashMap::new() }
    }

    pub fn add_mapping(&mut self, handle: &str, address: &str) -> bool {
        if self.map.contains_key(handle) {
            false
        } else {
            self.map.insert(handle.to_string(), address.to_string());
            true
        }
    }

    pub fn get_address(&self, handle: &str) -> Option<&String> {
        self.map.get(handle)
    }

    pub fn save_to_file(&self, path: &str) {
        let json = serde_json::to_string_pretty(&self).unwrap();
        fs::write(path, json).unwrap();
    }

    pub fn load_from_file(path: &str) -> Self {
        if Path::new(path).exists() {
            let data = fs::read_to_string(path).unwrap();
            serde_json::from_str(&data).unwrap()
        } else {
            AddressBook::new()
        }
    }
}
