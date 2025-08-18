//! DNS CLI for IPPAN on-chain DNS system

use crate::dns::{types::*, apply::*};
use crate::Result;
use clap::{Arg, ArgMatches, Command};
use serde_json::json;
use std::str::FromStr;

/// DNS CLI commands
pub struct DnsCli;

impl DnsCli {
    /// Create the DNS CLI command
    pub fn command() -> Command {
        Command::new("dns")
            .about("DNS zone management commands")
            .subcommand_negates_reqs(true)
            .subcommand(
                Command::new("zone")
                    .about("Zone management")
                    .subcommand(
                        Command::new("upsert")
                            .about("Create or update a DNS record set")
                            .arg(Arg::new("domain").required(true).help("Domain name"))
                            .arg(Arg::new("name").required(true).help("Record name (e.g., @, www)"))
                            .arg(Arg::new("type").required(true).help("Record type (A, AAAA, CNAME, etc.)"))
                            .arg(Arg::new("ttl").required(true).help("Time to live in seconds"))
                            .arg(Arg::new("records").required(true).help("Record values (JSON array)"))
                    )
                    .subcommand(
                        Command::new("delete")
                            .about("Delete a DNS record set")
                            .arg(Arg::new("domain").required(true).help("Domain name"))
                            .arg(Arg::new("name").required(true).help("Record name"))
                            .arg(Arg::new("type").required(true).help("Record type"))
                    )
                    .subcommand(
                        Command::new("get")
                            .about("Get zone information")
                            .arg(Arg::new("domain").required(true).help("Domain name"))
                    )
                    .subcommand(
                        Command::new("resolve")
                            .about("Resolve a DNS query")
                            .arg(Arg::new("name").required(true).help("Query name"))
                            .arg(Arg::new("type").required(true).help("Query type"))
                    )
            )
    }

    /// Handle DNS CLI commands
    pub fn handle(matches: &ArgMatches) -> Result<()> {
        match matches.subcommand() {
            Some(("zone", zone_matches)) => {
                Self::handle_zone_commands(zone_matches)?;
            }
            _ => {
                println!("Use 'dns zone --help' for zone management commands");
            }
        }
        Ok(())
    }

    /// Handle zone subcommands
    fn handle_zone_commands(matches: &ArgMatches) -> Result<()> {
        match matches.subcommand() {
            Some(("upsert", upsert_matches)) => {
                Self::handle_upsert(upsert_matches)?;
            }
            Some(("delete", delete_matches)) => {
                Self::handle_delete(delete_matches)?;
            }
            Some(("get", get_matches)) => {
                Self::handle_get(get_matches)?;
            }
            Some(("resolve", resolve_matches)) => {
                Self::handle_resolve(resolve_matches)?;
            }
            _ => {
                println!("Use 'dns zone --help' for available zone commands");
            }
        }
        Ok(())
    }

    /// Handle upsert command
    fn handle_upsert(matches: &ArgMatches) -> Result<()> {
        let domain = matches.get_one::<String>("domain").unwrap();
        let name = matches.get_one::<String>("name").unwrap();
        let rtype_str = matches.get_one::<String>("type").unwrap();
        let ttl_str = matches.get_one::<String>("ttl").unwrap();
        let records_str = matches.get_one::<String>("records").unwrap();

        // Parse record type
        let rtype = Rtype::from_str(rtype_str)
            .map_err(|e| anyhow::anyhow!("Invalid record type: {}", e))?;

        // Parse TTL
        let ttl = ttl_str.parse::<u32>()
            .map_err(|e| anyhow::anyhow!("Invalid TTL: {}", e))?;

        // Parse records
        let records: Vec<serde_json::Value> = serde_json::from_str(records_str)
            .map_err(|e| anyhow::anyhow!("Invalid records JSON: {}", e))?;

        // Create zone update transaction
        let op = ZoneOp::UpsertRrset {
            name: name.clone(),
            rtype,
            ttl,
            records,
        };

        let tx = ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: domain.clone(),
            nonce: 1, // TODO: Get actual nonce
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100, // TODO: Calculate actual fee
            memo: Some("DNS zone update via CLI".to_string()),
            sig: vec![], // TODO: Sign transaction
        };

        println!("Created zone update transaction for {}: {} {}", domain, name, rtype_str);
        println!("Transaction: {}", serde_json::to_string_pretty(&tx)?);

        Ok(())
    }

    /// Handle delete command
    fn handle_delete(matches: &ArgMatches) -> Result<()> {
        let domain = matches.get_one::<String>("domain").unwrap();
        let name = matches.get_one::<String>("name").unwrap();
        let rtype_str = matches.get_one::<String>("type").unwrap();

        // Parse record type
        let rtype = Rtype::from_str(rtype_str)
            .map_err(|e| anyhow::anyhow!("Invalid record type: {}", e))?;

        // Create zone update transaction
        let op = ZoneOp::DeleteRrset {
            name: name.clone(),
            rtype,
        };

        let tx = ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: domain.clone(),
            nonce: 1, // TODO: Get actual nonce
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100, // TODO: Calculate actual fee
            memo: Some("DNS zone delete via CLI".to_string()),
            sig: vec![], // TODO: Sign transaction
        };

        println!("Created zone delete transaction for {}: {} {}", domain, name, rtype_str);
        println!("Transaction: {}", serde_json::to_string_pretty(&tx)?);

        Ok(())
    }

    /// Handle get command
    fn handle_get(matches: &ArgMatches) -> Result<()> {
        let domain = matches.get_one::<String>("domain").unwrap();

        println!("Getting zone information for: {}", domain);
        println!("This would query the blockchain for zone data");
        // TODO: Implement actual zone query

        Ok(())
    }

    /// Handle resolve command
    fn handle_resolve(matches: &ArgMatches) -> Result<()> {
        let name = matches.get_one::<String>("name").unwrap();
        let rtype_str = matches.get_one::<String>("type").unwrap();

        // Parse record type
        let _rtype = Rtype::from_str(rtype_str)
            .map_err(|e| anyhow::anyhow!("Invalid record type: {}", e))?;

        println!("Resolving: {} {}", name, rtype_str);
        println!("This would query the DNS resolver");
        // TODO: Implement actual DNS resolution

        Ok(())
    }
}

/// Example usage functions for common DNS operations
pub struct DnsExamples;

impl DnsExamples {
    /// Create an example A record transaction
    pub fn create_a_record_example() -> ZoneUpdateTx {
        let op = ZoneOp::UpsertRrset {
            name: "www".to_string(),
            rtype: Rtype::A,
            ttl: 300,
            records: vec![json!("192.168.1.1")],
        };

        ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: "example.ipn".to_string(),
            nonce: 1,
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100,
            memo: Some("Example A record".to_string()),
            sig: vec![],
        }
    }

    /// Create an example MX record transaction
    pub fn create_mx_record_example() -> ZoneUpdateTx {
        let op = ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::MX,
            ttl: 3600,
            records: vec![
                json!({
                    "preference": 10,
                    "host": "mx1.example.com."
                }),
                json!({
                    "preference": 20,
                    "host": "mx2.example.com."
                })
            ],
        };

        ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: "example.ipn".to_string(),
            nonce: 2,
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100,
            memo: Some("Example MX records".to_string()),
            sig: vec![],
        }
    }

    /// Create an example TXT record transaction
    pub fn create_txt_record_example() -> ZoneUpdateTx {
        let op = ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::TXT,
            ttl: 300,
            records: vec![
                json!(["v=spf1 include:_spf.example.com ~all"]),
                json!(["google-site-verification=abc123"]),
            ],
        };

        ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: "example.ipn".to_string(),
            nonce: 3,
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100,
            memo: Some("Example TXT records".to_string()),
            sig: vec![],
        }
    }

    /// Create an example HTTPS/SVCB record transaction
    pub fn create_https_record_example() -> ZoneUpdateTx {
        let op = ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::HTTPS,
            ttl: 300,
            records: vec![
                json!({
                    "priority": 1,
                    "target": "svc.example.net.",
                    "params": {
                        "alpn": ["h2", "http/1.1"],
                        "port": 443,
                        "ipv4hint": ["1.2.3.4"]
                    }
                })
            ],
        };

        ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: "example.ipn".to_string(),
            nonce: 4,
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100,
            memo: Some("Example HTTPS record".to_string()),
            sig: vec![],
        }
    }

    /// Create an example CONTENT record transaction
    pub fn create_content_record_example() -> ZoneUpdateTx {
        let op = ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::CONTENT,
            ttl: 300,
            records: vec![
                json!({
                    "kind": "ipfs",
                    "hash": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
                }),
                json!({
                    "kind": "dht",
                    "hashtimer": "0x8fb5f8023c2b3f1a9e7d4c6b2a8f9e1d3c5b7a9f2e4d6c8b1a3f5e7d9c2b4a6f8"
                })
            ],
        };

        ZoneUpdateTx {
            r#type: "zone_update".to_string(),
            domain: "example.ipn".to_string(),
            nonce: 5,
            ops: vec![op],
            updated_at_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            fee_nano: 100,
            memo: Some("Example CONTENT records".to_string()),
            sig: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_examples() {
        // Test A record example
        let a_tx = DnsExamples::create_a_record_example();
        assert_eq!(a_tx.domain, "example.ipn");
        assert_eq!(a_tx.ops.len(), 1);
        
        if let ZoneOp::UpsertRrset { name, rtype, ttl, records } = &a_tx.ops[0] {
            assert_eq!(name, "www");
            assert!(matches!(rtype, Rtype::A));
            assert_eq!(*ttl, 300);
            assert_eq!(records.len(), 1);
        } else {
            panic!("Expected UPSERT_RRSET operation");
        }

        // Test MX record example
        let mx_tx = DnsExamples::create_mx_record_example();
        assert_eq!(mx_tx.ops.len(), 1);
        
        if let ZoneOp::UpsertRrset { rtype, records, .. } = &mx_tx.ops[0] {
            assert!(matches!(rtype, Rtype::MX));
            assert_eq!(records.len(), 2);
        } else {
            panic!("Expected UPSERT_RRSET operation");
        }
    }
}
