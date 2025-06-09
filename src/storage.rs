use crate::blockchain::Blockchain;
use std::fs::File;
use std::io::{Read, Write};
use serde_json;

pub fn save_chain(chain: &Blockchain, filename: &str) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(chain).unwrap();
    let mut file = File::create(filename)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

pub fn load_chain(filename: &str) -> Option<Blockchain> {
    let mut file = File::open(filename).ok()?;
    let mut data = String::new();
    file.read_to_string(&mut data).ok()?;
    serde_json::from_str(&data).ok()
}
