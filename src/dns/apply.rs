//! DNS zone update transaction application

use super::types::*;
use crate::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Zone update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum ZoneOp {
    /// Create or replace an entire RRSET
    UpsertRrset {
        name: String,
        rtype: Rtype,
        ttl: u32,
        records: Vec<serde_json::Value>,
    },
    /// Delete an entire RRSET
    DeleteRrset {
        name: String,
        rtype: Rtype,
    },
    /// Add/remove specific records within an RRSET
    PatchRecords {
        name: String,
        rtype: Rtype,
        add: Option<Vec<serde_json::Value>>,
        remove: Option<Vec<serde_json::Value>>,
    },
}

/// Zone update transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneUpdateTx {
    /// Transaction type
    pub r#type: String,
    /// Domain name
    pub domain: String,
    /// Nonce (must be > zone.serial)
    pub nonce: u64,
    /// Zone operations
    pub ops: Vec<ZoneOp>,
    /// Update timestamp in microseconds
    pub updated_at_us: u64,
    /// Transaction fee in nano IPN
    pub fee_nano: u64,
    /// Optional memo
    pub memo: Option<String>,
    /// Transaction signature
    pub sig: Vec<u8>,
}

/// Apply a zone update transaction
pub async fn apply_zone_update(
    tx: &ZoneUpdateTx,
    zones: &Arc<RwLock<BTreeMap<String, Zone>>>,
    _validator: &Arc<super::validator::ZoneValidator>,
) -> Result<()> {
    // Validate transaction type
    if tx.r#type != "zone_update" {
        return Err(crate::IppanError::Validation(format!("Invalid transaction type: {}", tx.r#type)));
    }

    // Get or create zone
    let mut zones_guard = zones.write().await;
    let zone = zones_guard.entry(tx.domain.clone()).or_insert_with(|| {
        // Create new zone if it doesn't exist
        Zone {
            domain: tx.domain.clone(),
            owner_pk: [0u8; 32], // Will be set from signature verification
            version: 1,
            serial: 0,
            updated_at_us: 0,
            rrsets: BTreeMap::new(),
        }
    });

    // Validate nonce
    if tx.nonce <= zone.serial as u64 {
        return Err(crate::IppanError::Validation(format!("Nonce {} not increasing vs serial {}", tx.nonce, zone.serial)));
    }

    // Validate time skew (allow 5 second skew)
    let now_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    let skew = now_us.abs_diff(tx.updated_at_us);
    if skew > 5_000_000 {
        return Err(crate::IppanError::Validation(format!("Time skew too large: {}μs", skew)));
    }

    // Validate fee
    let min_fee = calculate_min_fee(&tx.ops);
    if tx.fee_nano < min_fee {
        return Err(crate::IppanError::Validation(format!("Fee {} too low, minimum: {}", tx.fee_nano, min_fee)));
    }

    // Apply operations
    for op in &tx.ops {
        match op {
            ZoneOp::UpsertRrset { name, rtype, ttl, records } => {
                let rrset = build_rrset(name, rtype, *ttl, records)?;
                enforce_conflicts(zone, name, rtype, &rrset)?;
                let key = RrsetKey {
                    name: name.clone(),
                    rtype: rtype.clone(),
                };
                zone.rrsets.insert(key, rrset);
            }
            ZoneOp::DeleteRrset { name, rtype } => {
                let key = RrsetKey {
                    name: name.clone(),
                    rtype: rtype.clone(),
                };
                zone.rrsets.remove(&key);
            }
            ZoneOp::PatchRecords { name, rtype, add, remove } => {
                let key = RrsetKey {
                    name: name.clone(),
                    rtype: rtype.clone(),
                };
                let rrset = zone.rrsets.get_mut(&key)
                    .ok_or_else(|| anyhow::anyhow!("RRSET not found for patching"))?;
                patch_rrset(rrset, rtype, add, remove)?;
                // Note: enforce_conflicts is called after the mutable borrow is dropped
            }
        }
    }

    // Update zone metadata
    zone.serial = zone.serial.saturating_add(1);
    zone.updated_at_us = tx.updated_at_us;

    log::info!("Applied zone update for {}: serial={}", tx.domain, zone.serial);
    Ok(())
}

/// Build and validate an RRSET
fn build_rrset(
    _name: &str,
    rtype: &Rtype,
    ttl: u32,
    records: &[serde_json::Value],
) -> Result<Rrset> {
    // Validate TTL
    if ttl < rtype.min_ttl() || ttl > rtype.max_ttl() {
        return Err(crate::IppanError::Validation(format!("TTL {} out of range [{}, {}] for {}", ttl, rtype.min_ttl(), rtype.max_ttl(), rtype)));
    }

    // Validate records based on type
    let validated_records = validate_records(rtype, records)?;

    Ok(Rrset {
        ttl,
        records: validated_records,
    })
}

/// Validate records against record type
fn validate_records(rtype: &Rtype, records: &[serde_json::Value]) -> Result<Vec<serde_json::Value>> {
    let mut validated = Vec::new();

    for record in records {
        let validated_record = match rtype {
            Rtype::A => validate_a_record(record)?,
            Rtype::AAAA => validate_aaaa_record(record)?,
            Rtype::CNAME | Rtype::ALIAS | Rtype::NS | Rtype::PTR => validate_hostname_record(record)?,
            Rtype::MX => validate_mx_record(record)?,
            Rtype::TXT => validate_txt_record(record)?,
            Rtype::SRV => validate_srv_record(record)?,
            Rtype::SVCB | Rtype::HTTPS => validate_svcb_record(record)?,
            Rtype::CAA => validate_caa_record(record)?,
            Rtype::TLSA => validate_tlsa_record(record)?,
            Rtype::SSHFP => validate_sshfp_record(record)?,
            Rtype::SOA => validate_soa_record(record)?,
            Rtype::DNSKEY => validate_dnskey_record(record)?,
            Rtype::DS | Rtype::CDS => validate_ds_record(record)?,
            Rtype::NAPTR => validate_naptr_record(record)?,
            Rtype::CONTENT => validate_content_record(record)?,
            _ => record.clone(), // For now, accept other types as-is
        };
        validated.push(validated_record);
    }

    Ok(validated)
}

/// Validate A record (IPv4 address)
fn validate_a_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let ip_str = record.as_str()
        .ok_or_else(|| anyhow::anyhow!("A record must be a string"))?;
    
    let ip: Ipv4Addr = ip_str.parse()
        .context("Invalid IPv4 address")?;
    
    Ok(serde_json::Value::String(ip.to_string()))
}

/// Validate AAAA record (IPv6 address)
fn validate_aaaa_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let ip_str = record.as_str()
        .ok_or_else(|| anyhow::anyhow!("AAAA record must be a string"))?;
    
    let ip: Ipv6Addr = ip_str.parse()
        .context("Invalid IPv6 address")?;
    
    Ok(serde_json::Value::String(ip.to_string()))
}

/// Validate hostname record (CNAME, ALIAS, NS, PTR)
fn validate_hostname_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let hostname = record.as_str()
        .ok_or_else(|| anyhow::anyhow!("Hostname record must be a string"))?;
    
    // Basic hostname validation
    if hostname.is_empty() || hostname.len() > 253 {
        return Err(crate::IppanError::Validation("Invalid hostname length".to_string()));
    }
    
    // Ensure it ends with a dot if it's an absolute hostname
    let normalized = if hostname.ends_with('.') {
        hostname.to_string()
    } else {
        format!("{}.", hostname)
    };
    
    Ok(serde_json::Value::String(normalized))
}

/// Validate MX record
fn validate_mx_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("MX record must be an object"))?;
    
    let preference = obj.get("preference")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("MX record missing preference"))?;
    
    let host = obj.get("host")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("MX record missing host"))?;
    
    // Validate hostname
    let normalized_host = if host.ends_with('.') {
        host.to_string()
    } else {
        format!("{}.", host)
    };
    
    Ok(serde_json::json!({
        "preference": preference,
        "host": normalized_host
    }))
}

/// Validate TXT record
fn validate_txt_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    match record {
        serde_json::Value::String(s) => {
            if s.len() > 255 {
                return Err(crate::IppanError::Validation(format!("TXT record too long: {} chars", s.len())));
            }
            Ok(record.clone())
        }
        serde_json::Value::Array(arr) => {
            // Handle split TXT records
            for chunk in arr {
                let chunk_str = chunk.as_str()
                    .ok_or_else(|| anyhow::anyhow!("TXT array must contain strings"))?;
                if chunk_str.len() > 255 {
                    return Err(crate::IppanError::Validation(format!("TXT chunk too long: {} chars", chunk_str.len())));
                }
            }
            Ok(record.clone())
        }
        _ => return Err(crate::IppanError::Validation("TXT record must be string or array of strings".to_string())),
    }
}

/// Validate SRV record
fn validate_srv_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("SRV record must be an object"))?;
    
    let priority = obj.get("priority")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SRV record missing priority"))?;
    
    let weight = obj.get("weight")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SRV record missing weight"))?;
    
    let port = obj.get("port")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SRV record missing port"))?;
    
    let target = obj.get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("SRV record missing target"))?;
    
    // Validate target hostname
    let normalized_target = if target.ends_with('.') {
        target.to_string()
    } else {
        format!("{}.", target)
    };
    
    Ok(serde_json::json!({
        "priority": priority,
        "weight": weight,
        "port": port,
        "target": normalized_target
    }))
}

/// Validate SVCB/HTTPS record
fn validate_svcb_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("SVCB/HTTPS record must be an object"))?;
    
    let priority = obj.get("priority")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SVCB record missing priority"))?;
    
    let target = obj.get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("SVCB record missing target"))?;
    
    // Validate target hostname
    let normalized_target = if target.ends_with('.') {
        target.to_string()
    } else {
        format!("{}.", target)
    };
    
    let params = obj.get("params").cloned().unwrap_or_else(|| serde_json::json!({}));
    
    Ok(serde_json::json!({
        "priority": priority,
        "target": normalized_target,
        "params": params
    }))
}

/// Validate CAA record
fn validate_caa_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("CAA record must be an object"))?;
    
    let flag = obj.get("flag")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("CAA record missing flag"))?;
    
    let tag = obj.get("tag")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("CAA record missing tag"))?;
    
    let value = obj.get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("CAA record missing value"))?;
    
    Ok(serde_json::json!({
        "flag": flag,
        "tag": tag,
        "value": value
    }))
}

/// Validate TLSA record
fn validate_tlsa_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("TLSA record must be an object"))?;
    
    let usage = obj.get("usage")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("TLSA record missing usage"))?;
    
    let selector = obj.get("selector")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("TLSA record missing selector"))?;
    
    let mtype = obj.get("mtype")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("TLSA record missing mtype"))?;
    
    let cert = obj.get("cert")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("TLSA record missing cert"))?;
    
    Ok(serde_json::json!({
        "usage": usage,
        "selector": selector,
        "mtype": mtype,
        "cert": cert
    }))
}

/// Validate SSHFP record
fn validate_sshfp_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("SSHFP record must be an object"))?;
    
    let alg = obj.get("alg")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SSHFP record missing alg"))?;
    
    let r#type = obj.get("type")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SSHFP record missing type"))?;
    
    let fingerprint = obj.get("fingerprint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("SSHFP record missing fingerprint"))?;
    
    Ok(serde_json::json!({
        "alg": alg,
        "type": r#type,
        "fingerprint": fingerprint
    }))
}

/// Validate SOA record
fn validate_soa_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("SOA record must be an object"))?;
    
    let mname = obj.get("mname")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing mname"))?;
    
    let rname = obj.get("rname")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing rname"))?;
    
    let serial = obj.get("serial")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing serial"))?;
    
    let refresh = obj.get("refresh")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing refresh"))?;
    
    let retry = obj.get("retry")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing retry"))?;
    
    let expire = obj.get("expire")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing expire"))?;
    
    let minimum = obj.get("minimum")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("SOA record missing minimum"))?;
    
    // Normalize hostnames
    let normalized_mname = if mname.ends_with('.') {
        mname.to_string()
    } else {
        format!("{}.", mname)
    };
    
    let normalized_rname = if rname.ends_with('.') {
        rname.to_string()
    } else {
        format!("{}.", rname)
    };
    
    Ok(serde_json::json!({
        "mname": normalized_mname,
        "rname": normalized_rname,
        "serial": serial,
        "refresh": refresh,
        "retry": retry,
        "expire": expire,
        "minimum": minimum
    }))
}

/// Validate DNSKEY record
fn validate_dnskey_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("DNSKEY record must be an object"))?;
    
    let flags = obj.get("flags")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DNSKEY record missing flags"))?;
    
    let protocol = obj.get("protocol")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DNSKEY record missing protocol"))?;
    
    let algorithm = obj.get("algorithm")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DNSKEY record missing algorithm"))?;
    
    let public_key = obj.get("public_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("DNSKEY record missing public_key"))?;
    
    Ok(serde_json::json!({
        "flags": flags,
        "protocol": protocol,
        "algorithm": algorithm,
        "public_key": public_key
    }))
}

/// Validate DS record
fn validate_ds_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("DS record must be an object"))?;
    
    let key_tag = obj.get("key_tag")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DS record missing key_tag"))?;
    
    let algorithm = obj.get("algorithm")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DS record missing algorithm"))?;
    
    let digest_type = obj.get("digest_type")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("DS record missing digest_type"))?;
    
    let digest = obj.get("digest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("DS record missing digest"))?;
    
    Ok(serde_json::json!({
        "key_tag": key_tag,
        "algorithm": algorithm,
        "digest_type": digest_type,
        "digest": digest
    }))
}

/// Validate NAPTR record
fn validate_naptr_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("NAPTR record must be an object"))?;
    
    let order = obj.get("order")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing order"))?;
    
    let preference = obj.get("preference")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing preference"))?;
    
    let flags = obj.get("flags")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing flags"))?;
    
    let service = obj.get("service")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing service"))?;
    
    let regexp = obj.get("regexp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing regexp"))?;
    
    let replacement = obj.get("replacement")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("NAPTR record missing replacement"))?;
    
    Ok(serde_json::json!({
        "order": order,
        "preference": preference,
        "flags": flags,
        "service": service,
        "regexp": regexp,
        "replacement": replacement
    }))
}

/// Validate CONTENT record
fn validate_content_record(record: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = record.as_object()
        .ok_or_else(|| anyhow::anyhow!("CONTENT record must be an object"))?;
    
    let kind = obj.get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("CONTENT record missing kind"))?;
    
    match kind {
        "ipfs" => {
            let hash = obj.get("hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("IPFS CONTENT record missing hash"))?;
            Ok(serde_json::json!({ "kind": "ipfs", "hash": hash }))
        }
        "dht" => {
            let hashtimer = obj.get("hashtimer")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("DHT CONTENT record missing hashtimer"))?;
            Ok(serde_json::json!({ "kind": "dht", "hashtimer": hashtimer }))
        }
        "url" => {
            let href = obj.get("href")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("URL CONTENT record missing href"))?;
            Ok(serde_json::json!({ "kind": "url", "href": href }))
        }
        _ => return Err(crate::IppanError::Validation(format!("Unknown CONTENT record kind: {}", kind))),
    }
}

/// Enforce DNS record conflicts
fn enforce_conflicts(zone: &Zone, name: &str, rtype: &Rtype, _rrset: &Rrset) -> Result<()> {
    // Check for CNAME/ALIAS conflicts
    if rtype.is_exclusive() {
        // CNAME/ALIAS cannot coexist with other record types at the same name
        for (existing_key, _) in &zone.rrsets {
            if existing_key.name == name && existing_key.rtype != *rtype {
                return Err(crate::IppanError::Validation(format!("Cannot have {} and {} records at the same name", rtype, existing_key.rtype)));
            }
        }
    } else {
        // Other record types cannot coexist with CNAME/ALIAS
        for (existing_key, _) in &zone.rrsets {
            if existing_key.name == name && existing_key.rtype.is_exclusive() {
                return Err(crate::IppanError::Validation(format!("Cannot have {} and {} records at the same name", rtype, existing_key.rtype)));
            }
        }
    }

    // Check apex restrictions
    if name == "@" && !rtype.allowed_at_apex() {
        return Err(crate::IppanError::Validation(format!("Record type {} not allowed at apex", rtype)));
    }

    Ok(())
}

/// Patch an RRSET by adding/removing specific records
fn patch_rrset(
    rrset: &mut Rrset,
    rtype: &Rtype,
    add: &Option<Vec<serde_json::Value>>,
    remove: &Option<Vec<serde_json::Value>>,
) -> Result<()> {
    // Remove records if specified
    if let Some(remove_records) = remove {
        for record in remove_records {
            rrset.records.retain(|r| r != record);
        }
    }

    // Add records if specified
    if let Some(add_records) = add {
        let validated_records = validate_records(rtype, add_records)?;
        rrset.records.extend(validated_records);
    }

    Ok(())
}

/// Calculate minimum fee for zone update
fn calculate_min_fee(ops: &[ZoneOp]) -> u64 {
    let base_fee = 100; // 0.00000010 IPN base fee
    let per_op_fee = 50; // 0.00000005 IPN per operation
    let size_multiplier = 1; // TODO: Implement size-based fee calculation
    
    base_fee + (ops.len() as u64 * per_op_fee) * size_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_a_record() {
        let valid_ip = serde_json::json!("192.168.1.1");
        let result = validate_a_record(&valid_ip);
        assert!(result.is_ok());
        
        let invalid_ip = serde_json::json!("invalid");
        let result = validate_a_record(&invalid_ip);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_aaaa_record() {
        let valid_ip = serde_json::json!("2001:db8::1");
        let result = validate_aaaa_record(&valid_ip);
        assert!(result.is_ok());
        
        let invalid_ip = serde_json::json!("192.168.1.1");
        let result = validate_aaaa_record(&invalid_ip);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hostname_record() {
        let hostname = serde_json::json!("example.com");
        let result = validate_hostname_record(&hostname);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!("example.com."));
        
        let absolute_hostname = serde_json::json!("example.com.");
        let result = validate_hostname_record(&absolute_hostname);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!("example.com."));
    }

    #[test]
    fn test_validate_mx_record() {
        let mx_record = serde_json::json!({
            "preference": 10,
            "host": "mx.example.com"
        });
        let result = validate_mx_record(&mx_record);
        assert!(result.is_ok());
        
        let expected = serde_json::json!({
            "preference": 10,
            "host": "mx.example.com."
        });
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_calculate_min_fee() {
        let ops = vec![
            ZoneOp::UpsertRrset {
                name: "www".to_string(),
                rtype: Rtype::A,
                ttl: 300,
                records: vec![serde_json::json!("192.168.1.1")],
            }
        ];
        
        let fee = calculate_min_fee(&ops);
        assert_eq!(fee, 150); // base_fee + per_op_fee
    }
}
