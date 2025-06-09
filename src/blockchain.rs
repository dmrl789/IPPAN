use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub domains: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub owner: String,
    pub expires: Option<u64>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    pub accounts: HashMap<String, Account>,
    pub domains: HashMap<String, Domain>,
    pub blocks: Vec<Block>,
    pub tx_pool: Vec<Transaction>,
    pub hash_pointers: HashMap<String, (String, Option<String>)>,
    // For anti-flood: track last announce by address
    pub last_announce: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub transactions: Vec<Transaction>,
    pub timestamp: u64,
    pub previous_hash: String,
    pub hash: String,
}

impl Block {
    pub fn calc_hash(&self) -> String {
        let tx_hashes: String = self.transactions.iter().map(|tx| tx.hash.clone()).collect();
        let block_data = format!("{}{}{}{}{}", self.height, tx_hashes, self.timestamp, self.previous_hash, "IPN");
        let mut hasher = Sha256::new();
        hasher.update(block_data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer { to: String, amount: u64 },
    RegisterDomain { domain: String, years: u32 },
    UpdateDomain { domain: String, new_target: Option<String> },
    StoreHash { hash: String, description: Option<String> },
    RenewDomain { domain: String, years: u32 },
    NodeAvailability { address: String, pubkey: String, timestamp: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub tx_type: TransactionType,
    pub nonce: u64,
    pub signature: String,
    pub timestamp: u64,
    pub hash: String,
}

impl Transaction {
    pub fn calc_hash(&self) -> String {
        let tx_data = format!("{:?}{:?}{:?}{:?}{:?}", self.from, self.tx_type, self.nonce, self.signature, self.timestamp);
        let mut hasher = Sha256::new();
        hasher.update(tx_data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

// -------- Blockchain Logic --------
impl Blockchain {
    pub fn register_domain(
        &mut self,
        address: &str,
        domain: &str,
        years: u32,
        current_block: u64,
    ) -> Result<(), String> {
        let full_domain = if domain.starts_with('@') {
            domain.to_string()
        } else {
            format!("@{}.ipn", domain)
        };

        if self.domains.contains_key(&full_domain) {
            return Err("Domain already registered.".to_string());
        }
        let mut fee = 10 * years as u64;
        if full_domain.ends_with(".cyborg")
            || full_domain.ends_with(".m")
            || full_domain.ends_with(".humanoid")
            || full_domain.ends_with(".iot")
        {
            fee = 100 * years as u64;
        }
        let acc = self.accounts.get_mut(address).ok_or("Sender not found")?;
        if acc.balance < fee {
            return Err("Insufficient balance.".to_string());
        }
        acc.balance -= fee;
        acc.domains.insert(full_domain.clone());

        self.domains.insert(
            full_domain.clone(),
            Domain {
                name: full_domain.clone(),
                owner: address.to_string(),
                expires: Some(current_block + (years as u64 * 52 * 7)),
                target: None,
            },
        );
        Ok(())
    }

    pub fn update_domain(
        &mut self,
        address: &str,
        domain: &str,
        new_target: Option<String>,
    ) -> Result<(), String> {
        let d = self.domains.get_mut(domain).ok_or("Domain not found")?;
        if d.owner != address {
            return Err("Not the domain owner.".to_string());
        }
        d.target = new_target;
        Ok(())
    }

    pub fn resolve_domain(&self, domain: &str) -> Option<&str> {
        self.domains.get(domain).map(|d| d.owner.as_str())
    }

    pub fn renew_domain(
        &mut self,
        address: &str,
        domain: &str,
        years: u32,
        current_block: u64,
    ) -> Result<(), String> {
        let dom = self.domains.get_mut(domain).ok_or("Domain not found")?;
        if dom.owner != address {
            return Err("Only the owner can renew the domain.".to_string());
        }
        let mut fee = 10 * years as u64;
        if domain.ends_with(".cyborg")
            || domain.ends_with(".m")
            || domain.ends_with(".humanoid")
            || domain.ends_with(".iot")
        {
            fee = 100 * years as u64;
        }
        let acc = self.accounts.get_mut(address).ok_or("Sender not found")?;
        if acc.balance < fee {
            return Err("Insufficient balance for renewal.".to_string());
        }
        acc.balance -= fee;
        dom.expires = Some(dom.expires.unwrap_or(current_block) + (years as u64 * 52 * 7));
        Ok(())
    }

    pub fn is_domain_expired(&self, domain: &str, current_block: u64) -> Result<bool, String> {
        let dom = self.domains.get(domain).ok_or("Domain not found")?;
        if let Some(expiry) = dom.expires {
            Ok(expiry < current_block)
        } else {
            Ok(false)
        }
    }
}
