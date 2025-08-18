//! DNS validator for IPPAN on-chain DNS system

use super::types::*;
use crate::Result;
use anyhow::Context;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Helper function to convert anyhow errors to IppanError
fn dns_error(msg: &str) -> crate::IppanError {
    crate::IppanError::Validation(msg.to_string())
}

/// Helper function to convert anyhow errors to IppanError with format
fn dns_error_fmt(msg: &str, args: std::fmt::Arguments) -> crate::IppanError {
    crate::IppanError::Validation(format!("{}: {}", msg, args))
}

/// DNS zone validator
pub struct ZoneValidator {
    /// Maximum records per zone
    max_records_per_zone: usize,
    /// Maximum record size in bytes
    _max_record_size: usize,
    /// Maximum zone size in bytes
    max_zone_size: usize,
}

impl ZoneValidator {
    /// Create a new zone validator
    pub fn new() -> Self {
        Self {
            max_records_per_zone: 1000,
            _max_record_size: 65535,
            max_zone_size: 1024 * 1024, // 1MB
        }
    }

    /// Validate a zone update transaction
    pub fn validate_zone_update(&self, tx: &super::apply::ZoneUpdateTx) -> Result<()> {
        // Validate transaction type
        if tx.r#type != "zone_update" {
            return Err(crate::IppanError::Validation(format!("Invalid transaction type: {}", tx.r#type)));
        }

        // Validate domain name
        self.validate_domain_name(&tx.domain)?;

        // Validate nonce
        if tx.nonce == 0 {
            return Err(dns_error("Nonce must be greater than 0"));
        }

        // Validate operations
        if tx.ops.is_empty() {
            return Err(dns_error("Zone update must contain at least one operation"));
        }

        if tx.ops.len() > 100 {
            return Err(dns_error_fmt("Too many operations", format_args!("{}", tx.ops.len())));
        }

        // Validate each operation
        for op in &tx.ops {
            self.validate_zone_operation(op)?;
        }

        // Validate fee
        if tx.fee_nano == 0 {
            return Err(dns_error("Transaction fee must be greater than 0"));
        }

        // Validate signature
        if tx.sig.is_empty() {
            return Err(dns_error("Transaction signature is required"));
        }

        Ok(())
    }

    /// Validate a zone operation
    pub fn validate_zone_operation(&self, op: &super::apply::ZoneOp) -> Result<()> {
        match op {
            super::apply::ZoneOp::UpsertRrset { name, rtype, ttl, records } => {
                self.validate_rrset_name(name)?;
                self.validate_rrset_records(rtype, records)?;
                self.validate_ttl(*ttl, rtype)?;
            }
            super::apply::ZoneOp::DeleteRrset { name, rtype: _ } => {
                self.validate_rrset_name(name)?;
            }
            super::apply::ZoneOp::PatchRecords { name, rtype, add, remove } => {
                self.validate_rrset_name(name)?;
                if let Some(add_records) = add {
                    self.validate_rrset_records(rtype, add_records)?;
                }
                if let Some(remove_records) = remove {
                    self.validate_rrset_records(rtype, remove_records)?;
                }
            }
        }

        Ok(())
    }

    /// Validate a domain name
    pub fn validate_domain_name(&self, domain: &str) -> Result<()> {
        if domain.is_empty() {
            return Err(dns_error("Domain name cannot be empty"));
        }

        if domain.len() > 253 {
            return Err(dns_error_fmt("Domain name too long", format_args!("{} chars", domain.len())));
        }

        // Check for valid characters
        for (i, ch) in domain.chars().enumerate() {
            if !ch.is_alphanumeric() && ch != '-' && ch != '.' {
                return Err(dns_error_fmt("Invalid character", format_args!("'{}' at position {}", ch, i)));
            }
        }

        // Check for consecutive dots
        if domain.contains("..") {
            return Err(dns_error("Domain name cannot contain consecutive dots"));
        }

        // Check for leading/trailing dots
        if domain.starts_with('.') || domain.ends_with('.') {
            return Err(dns_error("Domain name cannot start or end with a dot"));
        }

        // Check for valid TLD
        if !domain.contains('.') {
            return Err(dns_error("Domain name must contain at least one dot"));
        }

        // Validate label lengths
        for label in domain.split('.') {
            if label.is_empty() {
                return Err(dns_error("Domain name cannot contain empty labels"));
            }
            if label.len() > 63 {
                return Err(dns_error_fmt("Label too long", format_args!("{} chars", label.len())));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(dns_error("Label cannot start or end with a hyphen"));
            }
        }

        Ok(())
    }

    /// Validate an RRSET name
    pub fn validate_rrset_name(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(dns_error("RRSET name cannot be empty"));
        }

        // Special case for apex
        if name == "@" {
            return Ok(());
        }

        // Check for wildcard
        if name.starts_with('*') {
            if name.len() == 1 {
                return Ok(());
            }
            if !name.starts_with("*.") {
                return Err(dns_error("Wildcard must be '*' or '*.<label>'"));
            }
            let label = &name[2..];
            self.validate_label(label)?;
            return Ok(());
        }

        // Validate as regular label
        self.validate_label(name)
    }

    /// Validate a DNS label
    pub fn validate_label(&self, label: &str) -> Result<()> {
        if label.is_empty() {
            return Err(dns_error("Label cannot be empty"));
        }

        if label.len() > 63 {
            return Err(dns_error_fmt("Label too long", format_args!("{} chars", label.len())));
        }

        // Check for valid characters
        for (i, ch) in label.chars().enumerate() {
            if !ch.is_alphanumeric() && ch != '-' {
                return Err(dns_error_fmt("Invalid character", format_args!("'{}' at position {}", ch, i)));
            }
        }

        // Check for leading/trailing hyphens
        if label.starts_with('-') || label.ends_with('-') {
            return Err(dns_error("Label cannot start or end with a hyphen"));
        }

        Ok(())
    }

    /// Validate RRSET records
    pub fn validate_rrset_records(&self, rtype: &Rtype, records: &Vec<serde_json::Value>) -> Result<()> {
        if records.is_empty() {
            return Err(dns_error("RRSET must contain at least one record"));
        }

        if records.len() > 100 {
            return Err(dns_error_fmt("Too many records", format_args!("{}", records.len())));
        }

        // Validate each record
        for (i, record) in records.iter().enumerate() {
            self.validate_record(rtype, record)
                .map_err(|e| dns_error_fmt("Record validation failed", format_args!("{}: {}", i, e)))?;
        }

        Ok(())
    }

    /// Validate a single record
    pub fn validate_record(&self, rtype: &Rtype, record: &serde_json::Value) -> Result<()> {
        match rtype {
            Rtype::A => self.validate_a_record(record),
            Rtype::AAAA => self.validate_aaaa_record(record),
            Rtype::CNAME | Rtype::ALIAS | Rtype::NS | Rtype::PTR => self.validate_hostname_record(record),
            Rtype::MX => self.validate_mx_record(record),
            Rtype::TXT => self.validate_txt_record(record),
            Rtype::SRV => self.validate_srv_record(record),
            Rtype::SVCB | Rtype::HTTPS => self.validate_svcb_record(record),
            Rtype::CAA => self.validate_caa_record(record),
            Rtype::TLSA => self.validate_tlsa_record(record),
            Rtype::SSHFP => self.validate_sshfp_record(record),
            Rtype::SOA => self.validate_soa_record(record),
            Rtype::DNSKEY => self.validate_dnskey_record(record),
            Rtype::DS | Rtype::CDS => self.validate_ds_record(record),
            Rtype::NAPTR => self.validate_naptr_record(record),
            Rtype::CONTENT => self.validate_content_record(record),
            _ => Ok(()), // Accept other types for now
        }
    }

    /// Validate A record
    pub fn validate_a_record(&self, record: &serde_json::Value) -> Result<()> {
        let ip_str = record.as_str()
            .ok_or_else(|| dns_error("A record must be a string"))?;
        
        let ip: Ipv4Addr = ip_str.parse()
            .map_err(|_| dns_error("Invalid IPv4 address"))?;
        
        // Check for reserved addresses
        if ip.is_loopback() || ip.is_unspecified() || ip.is_broadcast() {
            return Err(dns_error_fmt("Reserved IPv4 address not allowed", format_args!("{}", ip)));
        }
        
        Ok(())
    }

    /// Validate AAAA record
    pub fn validate_aaaa_record(&self, record: &serde_json::Value) -> Result<()> {
        let ip_str = record.as_str()
            .ok_or_else(|| dns_error("AAAA record must be a string"))?;
        
        let ip: Ipv6Addr = ip_str.parse()
            .map_err(|_| dns_error("Invalid IPv6 address"))?;
        
        // Check for reserved addresses
        if ip.is_loopback() || ip.is_unspecified() {
            return Err(dns_error_fmt("Reserved IPv6 address not allowed", format_args!("{}", ip)));
        }
        
        Ok(())
    }

    /// Validate hostname record
    pub fn validate_hostname_record(&self, record: &serde_json::Value) -> Result<()> {
        let hostname = record.as_str()
            .ok_or_else(|| anyhow::anyhow!("Hostname record must be a string"))?;
        
        if hostname.is_empty() {
            return Err(dns_error("Hostname cannot be empty"));
        }
        
        if hostname.len() > 253 {
            return Err(dns_error_fmt("Hostname too long", format_args!("{} chars", hostname.len())));
        }
        
        // Validate each label
        for label in hostname.split('.') {
            if label.is_empty() {
                return Err(dns_error("Hostname cannot contain empty labels"));
            }
            if label.len() > 63 {
                return Err(dns_error_fmt("Hostname label too long", format_args!("{} chars", label.len())));
            }
        }
        
        Ok(())
    }

    /// Validate MX record
    pub fn validate_mx_record(&self, record: &serde_json::Value) -> Result<()> {
        let obj = record.as_object()
            .ok_or_else(|| anyhow::anyhow!("MX record must be an object"))?;
        
        let preference = obj.get("preference")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("MX record missing preference"))?;
        
        let host = obj.get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("MX record missing host"))?;
        
        // Validate preference (0-65535)
        if preference > 65535 {
            return Err(dns_error_fmt("MX preference out of range", format_args!("{}", preference)));
        }
        
        // Validate hostname
        self.validate_hostname_record(&serde_json::Value::String(host.to_string()))?;
        
        Ok(())
    }

    /// Validate TXT record
    pub fn validate_txt_record(&self, record: &serde_json::Value) -> Result<()> {
        match record {
            serde_json::Value::String(s) => {
                if s.len() > 255 {
                    return Err(dns_error_fmt("TXT record too long", format_args!("{} chars", s.len())));
                }
                Ok(())
            }
            serde_json::Value::Array(arr) => {
                // Handle split TXT records
                for chunk in arr {
                    let chunk_str = chunk.as_str()
                        .ok_or_else(|| anyhow::anyhow!("TXT array must contain strings"))?;
                    if chunk_str.len() > 255 {
                        return Err(dns_error_fmt("TXT chunk too long", format_args!("{} chars", chunk_str.len())));
                    }
                }
                Ok(())
            }
            _ => return Err(dns_error("TXT record must be string or array of strings")),
        }
    }

    /// Validate SRV record
    pub fn validate_srv_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate ranges
        if priority > 65535 {
            return Err(dns_error_fmt("SRV priority out of range", format_args!("{}", priority)));
        }
        if weight > 65535 {
            return Err(dns_error_fmt("SRV weight out of range", format_args!("{}", weight)));
        }
        if port == 0 || port > 65535 {
            return Err(dns_error_fmt("SRV port out of range", format_args!("{}", port)));
        }
        
        // Validate target hostname
        self.validate_hostname_record(&serde_json::Value::String(target.to_string()))?;
        
        Ok(())
    }

    /// Validate SVCB/HTTPS record
    pub fn validate_svcb_record(&self, record: &serde_json::Value) -> Result<()> {
        let obj = record.as_object()
            .ok_or_else(|| anyhow::anyhow!("SVCB/HTTPS record must be an object"))?;
        
        let priority = obj.get("priority")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SVCB record missing priority"))?;
        
        let target = obj.get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SVCB record missing target"))?;
        
        // Validate priority
        if priority > 65535 {
            return Err(dns_error_fmt("SVCB priority out of range", format_args!("{}", priority)));
        }
        
        // Validate target hostname
        self.validate_hostname_record(&serde_json::Value::String(target.to_string()))?;
        
        Ok(())
    }

    /// Validate CAA record
    pub fn validate_caa_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate flag (0-255)
        if flag > 255 {
            return Err(dns_error_fmt("CAA flag out of range", format_args!("{}", flag)));
        }
        
        // Validate tag
        if tag.is_empty() || tag.len() > 255 {
            return Err(dns_error_fmt("CAA tag invalid length", format_args!("{}", tag.len())));
        }
        
        // Validate value
        if value.len() > 255 {
            return Err(dns_error_fmt("CAA value too long", format_args!("{} chars", value.len())));
        }
        
        Ok(())
    }

    /// Validate TLSA record
    pub fn validate_tlsa_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate ranges
        if usage > 3 {
            return Err(dns_error_fmt("TLSA usage out of range", format_args!("{}", usage)));
        }
        if selector > 1 {
            return Err(dns_error_fmt("TLSA selector out of range", format_args!("{}", selector)));
        }
        if mtype > 2 {
            return Err(dns_error_fmt("TLSA mtype out of range", format_args!("{}", mtype)));
        }
        
        // Validate certificate data
        if cert.is_empty() {
            return Err(dns_error("TLSA certificate data cannot be empty"));
        }
        
        Ok(())
    }

    /// Validate SSHFP record
    pub fn validate_sshfp_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate ranges
        if alg > 4 {
            return Err(crate::IppanError::Validation(format!("SSHFP algorithm out of range: {}", alg)));
        }
        if r#type > 2 {
            return Err(crate::IppanError::Validation(format!("SSHFP type out of range: {}", r#type)));
        }
        
        // Validate fingerprint
        if fingerprint.is_empty() {
            return Err(crate::IppanError::Validation("SSHFP fingerprint cannot be empty".to_string()));
        }
        
        Ok(())
    }

    /// Validate SOA record
    pub fn validate_soa_record(&self, record: &serde_json::Value) -> Result<()> {
        let obj = record.as_object()
            .ok_or_else(|| anyhow::anyhow!("SOA record must be an object"))?;
        
        let mname = obj.get("mname")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SOA record missing mname"))?;
        
        let rname = obj.get("rname")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SOA record missing rname"))?;
        
        let _serial = obj.get("serial")
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
        
        let _minimum = obj.get("minimum")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SOA record missing minimum"))?;
        
        // Validate hostnames
        self.validate_hostname_record(&serde_json::Value::String(mname.to_string()))?;
        self.validate_hostname_record(&serde_json::Value::String(rname.to_string()))?;
        
        // Validate timing values
        if refresh == 0 {
            return Err(crate::IppanError::Validation("SOA refresh cannot be 0".to_string()));
        }
        if retry == 0 {
            return Err(crate::IppanError::Validation("SOA retry cannot be 0".to_string()));
        }
        if expire == 0 {
            return Err(crate::IppanError::Validation("SOA expire cannot be 0".to_string()));
        }
        
        Ok(())
    }

    /// Validate DNSKEY record
    pub fn validate_dnskey_record(&self, record: &serde_json::Value) -> Result<()> {
        let obj = record.as_object()
            .ok_or_else(|| anyhow::anyhow!("DNSKEY record must be an object"))?;
        
        let _flags = obj.get("flags")
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
        
        // Validate protocol (must be 3)
        if protocol != 3 {
            return Err(crate::IppanError::Validation(format!("DNSKEY protocol must be 3, got: {}", protocol)));
        }
        
        // Validate algorithm
        if algorithm > 255 {
            return Err(crate::IppanError::Validation(format!("DNSKEY algorithm out of range: {}", algorithm)));
        }
        
        // Validate public key
        if public_key.is_empty() {
            return Err(crate::IppanError::Validation("DNSKEY public key cannot be empty".to_string()));
        }
        
        Ok(())
    }

    /// Validate DS record
    pub fn validate_ds_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate ranges
        if key_tag > 65535 {
            return Err(crate::IppanError::Validation(format!("DS key_tag out of range: {}", key_tag)));
        }
        if algorithm > 255 {
            return Err(crate::IppanError::Validation(format!("DS algorithm out of range: {}", algorithm)));
        }
        if digest_type > 255 {
            return Err(crate::IppanError::Validation(format!("DS digest_type out of range: {}", digest_type)));
        }
        
        // Validate digest
        if digest.is_empty() {
            return Err(crate::IppanError::Validation("DS digest cannot be empty".to_string()));
        }
        
        Ok(())
    }

    /// Validate NAPTR record
    pub fn validate_naptr_record(&self, record: &serde_json::Value) -> Result<()> {
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
        
        // Validate ranges
        if order > 65535 {
            return Err(crate::IppanError::Validation(format!("NAPTR order out of range: {}", order)));
        }
        if preference > 65535 {
            return Err(crate::IppanError::Validation(format!("NAPTR preference out of range: {}", preference)));
        }
        
        // Validate string fields
        if flags.len() > 255 {
            return Err(crate::IppanError::Validation(format!("NAPTR flags too long: {} chars", flags.len())));
        }
        if service.len() > 255 {
            return Err(crate::IppanError::Validation(format!("NAPTR service too long: {} chars", service.len())));
        }
        if regexp.len() > 255 {
            return Err(crate::IppanError::Validation(format!("NAPTR regexp too long: {} chars", regexp.len())));
        }
        if replacement.len() > 255 {
            return Err(crate::IppanError::Validation(format!("NAPTR replacement too long: {} chars", replacement.len())));
        }
        
        Ok(())
    }

    /// Validate CONTENT record
    pub fn validate_content_record(&self, record: &serde_json::Value) -> Result<()> {
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
                if hash.is_empty() {
                    return Err(crate::IppanError::Validation("IPFS hash cannot be empty".to_string()));
                }
            }
            "dht" => {
                let hashtimer = obj.get("hashtimer")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("DHT CONTENT record missing hashtimer"))?;
                if hashtimer.is_empty() {
                    return Err(crate::IppanError::Validation("DHT hashtimer cannot be empty".to_string()));
                }
            }
            "url" => {
                let href = obj.get("href")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("URL CONTENT record missing href"))?;
                if href.is_empty() {
                    return Err(crate::IppanError::Validation("URL href cannot be empty".to_string()));
                }
                // Basic URL validation
                if !href.starts_with("http://") && !href.starts_with("https://") {
                    return Err(crate::IppanError::Validation("URL must start with http:// or https://".to_string()));
                }
            }
            _ => return Err(crate::IppanError::Validation(format!("Unknown CONTENT record kind: {}", kind))),
        }
        
        Ok(())
    }

    /// Validate TTL
    pub fn validate_ttl(&self, ttl: u32, rtype: &Rtype) -> Result<()> {
        let min_ttl = rtype.min_ttl();
        let max_ttl = rtype.max_ttl();
        
        if ttl < min_ttl || ttl > max_ttl {
            return Err(crate::IppanError::Validation(format!("TTL {} out of range [{}, {}] for {}", ttl, min_ttl, max_ttl, rtype)));
        }
        
        Ok(())
    }

    /// Validate zone size
    pub fn validate_zone_size(&self, zone: &Zone) -> Result<()> {
        let zone_json = serde_json::to_string(zone)
            .context("Failed to serialize zone for size validation")?;
        
        if zone_json.len() > self.max_zone_size {
            return Err(crate::IppanError::Validation(format!("Zone too large: {} bytes (max: {})", zone_json.len(), self.max_zone_size)));
        }
        
        if zone.rrsets.len() > self.max_records_per_zone {
            return Err(crate::IppanError::Validation(format!("Too many records in zone: {} (max: {})", zone.rrsets.len(), self.max_records_per_zone)));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain_name() {
        let validator = ZoneValidator::new();
        
        // Valid domains
        assert!(validator.validate_domain_name("example.ipn").is_ok());
        assert!(validator.validate_domain_name("www.example.ipn").is_ok());
        assert!(validator.validate_domain_name("sub.domain.example.ipn").is_ok());
        
        // Invalid domains
        assert!(validator.validate_domain_name("").is_err());
        assert!(validator.validate_domain_name("example").is_err()); // No TLD
        assert!(validator.validate_domain_name(".example.ipn").is_err()); // Leading dot
        assert!(validator.validate_domain_name("example.ipn.").is_err()); // Trailing dot
        assert!(validator.validate_domain_name("example..ipn").is_err()); // Consecutive dots
    }

    #[test]
    fn test_validate_a_record() {
        let validator = ZoneValidator::new();
        
        // Valid A records
        assert!(validator.validate_a_record(&serde_json::json!("192.168.1.1")).is_ok());
        assert!(validator.validate_a_record(&serde_json::json!("8.8.8.8")).is_ok());
        
        // Invalid A records
        assert!(validator.validate_a_record(&serde_json::json!("127.0.0.1")).is_err()); // Loopback
        assert!(validator.validate_a_record(&serde_json::json!("0.0.0.0")).is_err()); // Unspecified
        assert!(validator.validate_a_record(&serde_json::json!("invalid")).is_err()); // Invalid IP
        assert!(validator.validate_a_record(&serde_json::json!(123)).is_err()); // Wrong type
    }

    #[test]
    fn test_validate_ttl() {
        let validator = ZoneValidator::new();
        
        // Valid TTLs
        assert!(validator.validate_ttl(300, &Rtype::A).is_ok());
        assert!(validator.validate_ttl(3600, &Rtype::TXT).is_ok());
        
        // Invalid TTLs
        assert!(validator.validate_ttl(30, &Rtype::A).is_err()); // Too low
        assert!(validator.validate_ttl(100000, &Rtype::A).is_err()); // Too high
    }
}
