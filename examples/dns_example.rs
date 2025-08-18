//! DNS Zone Management Example for IPPAN
//! 
//! This example demonstrates how to use the on-chain DNS system
//! to manage zone records for IPPAN domains.

use ippan::dns::{types::*, apply::*};
use ippan::api::dns_cli::DnsExamples;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IPPAN DNS Zone Management Example ===\n");

    // Example 1: Basic A record setup
    println!("1. Setting up basic A records for example.ipn");
    let a_record_tx = DnsExamples::create_a_record_example();
    println!("A Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&a_record_tx)?);
    println!();

    // Example 2: MX records for email
    println!("2. Setting up MX records for email");
    let mx_record_tx = DnsExamples::create_mx_record_example();
    println!("MX Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&mx_record_tx)?);
    println!();

    // Example 3: TXT records for SPF and verification
    println!("3. Setting up TXT records for SPF and verification");
    let txt_record_tx = DnsExamples::create_txt_record_example();
    println!("TXT Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&txt_record_tx)?);
    println!();

    // Example 4: HTTPS/SVCB records for modern service binding
    println!("4. Setting up HTTPS/SVCB records for modern service binding");
    let https_record_tx = DnsExamples::create_https_record_example();
    println!("HTTPS Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&https_record_tx)?);
    println!();

    // Example 5: CONTENT records for Web3 integration
    println!("5. Setting up CONTENT records for Web3 integration");
    let content_record_tx = DnsExamples::create_content_record_example();
    println!("CONTENT Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&content_record_tx)?);
    println!();

    // Example 6: Complex zone setup with multiple record types
    println!("6. Complex zone setup with multiple record types");
    let complex_zone_tx = create_complex_zone_example();
    println!("Complex Zone Transaction:");
    println!("{}", serde_json::to_string_pretty(&complex_zone_tx)?);
    println!();

    // Example 7: Service discovery with SRV records
    println!("7. Service discovery with SRV records");
    let srv_record_tx = create_srv_record_example();
    println!("SRV Record Transaction:");
    println!("{}", serde_json::to_string_pretty(&srv_record_tx)?);
    println!();

    // Example 8: Security records (CAA, TLSA, SSHFP)
    println!("8. Security records (CAA, TLSA, SSHFP)");
    let security_record_tx = create_security_records_example();
    println!("Security Records Transaction:");
    println!("{}", serde_json::to_string_pretty(&security_record_tx)?);
    println!();

    println!("=== DNS Zone Management Examples Complete ===");
    Ok(())
}

/// Create a complex zone with multiple record types
fn create_complex_zone_example() -> ZoneUpdateTx {
    let ops = vec![
        // Apex ALIAS record
        ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::ALIAS,
            ttl: 300,
            records: vec![json!("root.example.net.")],
        },
        // www A record
        ZoneOp::UpsertRrset {
            name: "www".to_string(),
            rtype: Rtype::A,
            ttl: 300,
            records: vec![json!("93.184.216.34")],
        },
        // api AAAA record
        ZoneOp::UpsertRrset {
            name: "api".to_string(),
            rtype: Rtype::AAAA,
            ttl: 300,
            records: vec![json!("2606:2800:220:1:248:1893:25c8:1946")],
        },
        // mail CNAME record
        ZoneOp::UpsertRrset {
            name: "mail".to_string(),
            rtype: Rtype::CNAME,
            ttl: 3600,
            records: vec![json!("mail.example.net.")],
        },
        // MX records
        ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::MX,
            ttl: 3600,
            records: vec![
                json!({
                    "preference": 10,
                    "host": "mx1.mailhost.com."
                }),
                json!({
                    "preference": 20,
                    "host": "mx2.mailhost.com."
                })
            ],
        },
        // TXT records for SPF and verification
        ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::TXT,
            ttl: 300,
            records: vec![
                json!(["v=spf1 include:_spf.example.com ~all"]),
                json!(["google-site-verification=abc123def456"]),
            ],
        },
        // NS records
        ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::NS,
            ttl: 86400,
            records: vec![
                json!("ns1.provider.net."),
                json!("ns2.provider.net."),
            ],
        },
    ];

    ZoneUpdateTx {
        r#type: "zone_update".to_string(),
        domain: "example.ipn".to_string(),
        nonce: 10,
        ops,
        updated_at_us: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
        fee_nano: 500,
        memo: Some("Complex zone setup".to_string()),
        sig: vec![],
    }
}

/// Create SRV records for service discovery
fn create_srv_record_example() -> ZoneUpdateTx {
    let ops = vec![
        // XMPP server
        ZoneOp::UpsertRrset {
            name: "_xmpp-server._tcp".to_string(),
            rtype: Rtype::SRV,
            ttl: 3600,
            records: vec![
                json!({
                    "priority": 10,
                    "weight": 5,
                    "port": 5269,
                    "target": "xmpp.example.net."
                })
            ],
        },
        // SIP service
        ZoneOp::UpsertRrset {
            name: "_sip._tcp".to_string(),
            rtype: Rtype::SRV,
            ttl: 3600,
            records: vec![
                json!({
                    "priority": 10,
                    "weight": 10,
                    "port": 5060,
                    "target": "sip.example.net."
                })
            ],
        },
        // LDAP service
        ZoneOp::UpsertRrset {
            name: "_ldap._tcp".to_string(),
            rtype: Rtype::SRV,
            ttl: 3600,
            records: vec![
                json!({
                    "priority": 10,
                    "weight": 5,
                    "port": 389,
                    "target": "ldap.example.net."
                })
            ],
        },
    ];

    ZoneUpdateTx {
        r#type: "zone_update".to_string(),
        domain: "example.ipn".to_string(),
        nonce: 11,
        ops,
        updated_at_us: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
        fee_nano: 300,
        memo: Some("Service discovery setup".to_string()),
        sig: vec![],
    }
}

/// Create security records (CAA, TLSA, SSHFP)
fn create_security_records_example() -> ZoneUpdateTx {
    let ops = vec![
        // CAA record for certificate authority authorization
        ZoneOp::UpsertRrset {
            name: "@".to_string(),
            rtype: Rtype::CAA,
            ttl: 3600,
            records: vec![
                json!({
                    "flag": 0,
                    "tag": "issue",
                    "value": "letsencrypt.org"
                }),
                json!({
                    "flag": 0,
                    "tag": "issuewild",
                    "value": "letsencrypt.org"
                })
            ],
        },
        // TLSA record for DANE (DNS-based Authentication of Named Entities)
        ZoneOp::UpsertRrset {
            name: "_443._tcp.www".to_string(),
            rtype: Rtype::TLSA,
            ttl: 3600,
            records: vec![
                json!({
                    "usage": 3,
                    "selector": 1,
                    "mtype": 1,
                    "cert": "ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890"
                })
            ],
        },
        // SSHFP record for SSH fingerprint
        ZoneOp::UpsertRrset {
            name: "ssh".to_string(),
            rtype: Rtype::SSHFP,
            ttl: 3600,
            records: vec![
                json!({
                    "alg": 1,
                    "type": 2,
                    "fingerprint": "1234567890ABCDEF1234567890ABCDEF12345678"
                })
            ],
        },
    ];

    ZoneUpdateTx {
        r#type: "zone_update".to_string(),
        domain: "example.ipn".to_string(),
        nonce: 12,
        ops,
        updated_at_us: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
        fee_nano: 300,
        memo: Some("Security records setup".to_string()),
        sig: vec![],
    }
}

/// Demonstrate DNS record validation
fn demonstrate_validation() {
    println!("=== DNS Record Validation Examples ===\n");

    // Valid A record
    let valid_a = json!("192.168.1.1");
    println!("Valid A record: {}", valid_a);

    // Invalid A record (reserved address)
    let invalid_a = json!("127.0.0.1");
    println!("Invalid A record (reserved): {}", invalid_a);

    // Valid AAAA record
    let valid_aaaa = json!("2001:db8::1");
    println!("Valid AAAA record: {}", valid_aaaa);

    // Valid MX record
    let valid_mx = json!({
        "preference": 10,
        "host": "mx.example.com."
    });
    println!("Valid MX record: {}", valid_mx);

    // Valid TXT record
    let valid_txt = json!(["v=spf1 include:_spf.example.com ~all"]);
    println!("Valid TXT record: {}", valid_txt);

    // Valid SRV record
    let valid_srv = json!({
        "priority": 10,
        "weight": 5,
        "port": 443,
        "target": "svc.example.net."
    });
    println!("Valid SRV record: {}", valid_srv);

    // Valid CAA record
    let valid_caa = json!({
        "flag": 0,
        "tag": "issue",
        "value": "letsencrypt.org"
    });
    println!("Valid CAA record: {}", valid_caa);

    // Valid CONTENT record
    let valid_content = json!({
        "kind": "ipfs",
        "hash": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
    });
    println!("Valid CONTENT record: {}", valid_content);
}

/// Demonstrate DNS resolution examples
fn demonstrate_resolution() {
    println!("\n=== DNS Resolution Examples ===\n");

    let resolution_examples = vec![
        ("www.example.ipn", "A"),
        ("api.example.ipn", "AAAA"),
        ("mail.example.ipn", "CNAME"),
        ("example.ipn", "MX"),
        ("example.ipn", "TXT"),
        ("_xmpp-server._tcp.example.ipn", "SRV"),
        ("example.ipn", "HTTPS"),
        ("example.ipn", "CONTENT"),
    ];

    for (name, rtype) in resolution_examples {
        println!("Resolving {} {} -> [would return records from blockchain]", name, rtype);
    }
}

/// Demonstrate zone management operations
fn demonstrate_zone_management() {
    println!("\n=== Zone Management Operations ===\n");

    println!("1. Zone Creation:");
    println!("   - Create new zone for domain");
    println!("   - Set initial SOA record");
    println!("   - Set NS records");

    println!("\n2. Record Management:");
    println!("   - Add/update records");
    println!("   - Delete records");
    println!("   - Patch specific records");

    println!("\n3. Zone Transfer:");
    println!("   - Transfer zone ownership");
    println!("   - Update zone metadata");

    println!("\n4. Zone Validation:");
    println!("   - Validate record formats");
    println!("   - Check for conflicts");
    println!("   - Verify TTL ranges");
}
