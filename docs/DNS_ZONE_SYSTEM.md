# IPPAN On-Chain DNS Zone System

## Overview

The IPPAN on-chain DNS zone system provides a decentralized, blockchain-based DNS solution that allows users to manage DNS records for their domains directly on the IPPAN blockchain. This system supports all modern DNS record types and provides a secure, immutable way to manage domain configurations.

## Key Features

- **On-Chain Storage**: All DNS records are stored on the IPPAN blockchain
- **Full DNS Support**: Supports all modern DNS record types (A, AAAA, MX, TXT, SRV, SVCB, HTTPS, etc.)
- **Web3 Integration**: Special CONTENT records for IPFS, DHT, and URL pointers
- **Atomic Updates**: Zone updates are applied atomically as blockchain transactions
- **Validation**: Comprehensive validation of all record types and formats
- **Resolution**: Fast DNS resolution with caching support
- **CLI Interface**: Command-line tools for zone management

## Architecture

### Core Components

1. **DNS System** (`src/dns/mod.rs`)
   - Main DNS system coordinator
   - Manages zone storage and resolution

2. **Zone Types** (`src/dns/types.rs`)
   - Defines DNS record types and data structures
   - Supports all standard DNS record types plus Web3 extensions

3. **Zone Application** (`src/dns/apply.rs`)
   - Handles zone update transaction processing
   - Validates and applies DNS record changes

4. **Zone Resolver** (`src/dns/resolver.rs`)
   - Resolves DNS queries against on-chain data
   - Provides caching and performance optimization

5. **Zone Validator** (`src/dns/validator.rs`)
   - Validates DNS records and zone configurations
   - Ensures data integrity and format compliance

## DNS Record Types

### Core Address & Name Mapping

- **A**: IPv4 addresses (e.g., `"192.168.1.1"`)
- **AAAA**: IPv6 addresses (e.g., `"2001:db8::1"`)
- **CNAME**: Canonical name records (non-apex only)
- **ALIAS**: Apex alias records for domain flattening

### Mail & Text Records

- **MX**: Mail exchange records with preference and host
- **TXT**: Text records for SPF, verification, and other purposes
- **SPF**: Legacy SPF records (stored as TXT)

### Service Discovery

- **SRV**: Service records for protocol-specific services
- **SVCB**: Service binding records for modern service discovery
- **HTTPS**: HTTPS service binding records

### Security & Certificates

- **CAA**: Certificate Authority Authorization
- **TLSA**: TLS authentication for DANE
- **SSHFP**: SSH fingerprint records

### DNSSEC Support

- **DNSKEY**: DNS key records
- **DS**: Delegation signer records
- **CDS/CDNSKEY**: Child delegation records

### Web3 Integration

- **CONTENT**: Content-addressed pointers for IPFS, DHT, and URLs

## Zone Update Transactions

### Transaction Structure

```json
{
  "type": "zone_update",
  "domain": "example.ipn",
  "nonce": 12,
  "ops": [
    {
      "op": "UPSERT_RRSET",
      "name": "@",
      "rtype": "ALIAS",
      "ttl": 300,
      "records": ["root.example.net."]
    }
  ],
  "updated_at_us": 1755327600123456,
  "fee_nano": 100,
  "memo": "initial zone setup",
  "sig": "ed25519:..."
}
```

### Operation Types

1. **UPSERT_RRSET**: Create or replace an entire record set
2. **DELETE_RRSET**: Delete an entire record set
3. **PATCH_RECORDS**: Add/remove specific records within a set

### Validation Rules

- **Nonce**: Must be strictly increasing per zone
- **TTL**: Must be within valid ranges (60-86400 seconds)
- **Records**: Must match the specified record type format
- **Conflicts**: CNAME/ALIAS records cannot coexist with other types
- **Apex**: CNAME records are not allowed at apex

## Usage Examples

### Basic A Record Setup

```rust
use ippan::dns::{types::*, apply::*};
use serde_json::json;

let op = ZoneOp::UPSERT_RRSET {
    name: "www".to_string(),
    rtype: Rtype::A,
    ttl: 300,
    records: vec![json!("192.168.1.1")],
};

let tx = ZoneUpdateTx {
    r#type: "zone_update".to_string(),
    domain: "example.ipn".to_string(),
    nonce: 1,
    ops: vec![op],
    updated_at_us: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64,
    fee_nano: 100,
    memo: Some("A record setup".to_string()),
    sig: vec![],
};
```

### MX Records for Email

```rust
let op = ZoneOp::UPSERT_RRSET {
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
};
```

### TXT Records for SPF

```rust
let op = ZoneOp::UPSERT_RRSET {
    name: "@".to_string(),
    rtype: Rtype::TXT,
    ttl: 300,
    records: vec![
        json!(["v=spf1 include:_spf.example.com ~all"]),
        json!(["google-site-verification=abc123"]),
    ],
};
```

### HTTPS/SVCB Records

```rust
let op = ZoneOp::UPSERT_RRSET {
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
```

### Web3 CONTENT Records

```rust
let op = ZoneOp::UPSERT_RRSET {
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
```

## CLI Usage

### Zone Management Commands

```bash
# Create or update a DNS record set
ippan-cli dns zone upsert example.ipn @ ALIAS --ttl 300 --records '["root.example.net."]'

# Add A record
ippan-cli dns zone upsert example.ipn www A --ttl 300 --records '["192.168.1.1"]'

# Add MX records
ippan-cli dns zone upsert example.ipn @ MX --ttl 3600 \
  --records '[{"preference":10,"host":"mx1.mailhost.com."},{"preference":20,"host":"mx2.mailhost.com."}]'

# Add TXT records
ippan-cli dns zone upsert example.ipn @ TXT --ttl 300 \
  --records '[["v=spf1 include:_spf.example.com ~all"]]'

# Delete a record set
ippan-cli dns zone delete example.ipn www A

# Get zone information
ippan-cli dns zone get example.ipn

# Resolve a DNS query
ippan-cli dns zone resolve www.example.ipn A
```

## API Endpoints

### Zone Management

```http
POST /v1/dns/zone/update
Content-Type: application/json

{
  "type": "zone_update",
  "domain": "example.ipn",
  "nonce": 12,
  "ops": [...],
  "updated_at_us": 1755327600123456,
  "fee_nano": 100,
  "memo": "zone update",
  "sig": "..."
}
```

### Zone Queries

```http
GET /v1/dns/zone/example.ipn
```

### DNS Resolution

```http
GET /v1/dns/resolve?name=www.example.ipn&type=A
```

## Record Type Reference

### A Record
```json
"192.168.1.1"
```

### AAAA Record
```json
"2001:db8::1"
```

### CNAME/ALIAS/NS/PTR Record
```json
"target.example.net."
```

### MX Record
```json
{
  "preference": 10,
  "host": "mx.example.net."
}
```

### TXT Record
```json
["v=spf1 include:_spf.example.com ~all"]
```

### SRV Record
```json
{
  "priority": 10,
  "weight": 5,
  "port": 443,
  "target": "svc.example.net."
}
```

### SVCB/HTTPS Record
```json
{
  "priority": 1,
  "target": "svc.example.net.",
  "params": {
    "alpn": ["h2", "http/1.1"],
    "port": 443,
    "ipv4hint": ["1.2.3.4"]
  }
}
```

### CAA Record
```json
{
  "flag": 0,
  "tag": "issue",
  "value": "letsencrypt.org"
}
```

### TLSA Record
```json
{
  "usage": 3,
  "selector": 1,
  "mtype": 1,
  "cert": "ABCDEF1234567890..."
}
```

### SSHFP Record
```json
{
  "alg": 1,
  "type": 2,
  "fingerprint": "1234567890ABCDEF..."
}
```

### SOA Record
```json
{
  "mname": "ns1.provider.net.",
  "rname": "hostmaster.example.ipn.",
  "serial": 2025081601,
  "refresh": 7200,
  "retry": 1200,
  "expire": 1209600,
  "minimum": 300
}
```

### DNSKEY Record
```json
{
  "flags": 257,
  "protocol": 3,
  "algorithm": 8,
  "public_key": "base64-encoded-key"
}
```

### DS Record
```json
{
  "key_tag": 12345,
  "algorithm": 8,
  "digest_type": 2,
  "digest": "hex-digest"
}
```

### CONTENT Record
```json
{
  "kind": "ipfs",
  "hash": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
}
```

## Best Practices

### Zone Management

1. **Use ALIAS for Apex**: Use ALIAS records instead of CNAME for apex domains
2. **Set Appropriate TTLs**: Use shorter TTLs for frequently changing records
3. **Validate Records**: Always validate record formats before submission
4. **Atomic Updates**: Group related changes in single transactions
5. **Backup Zones**: Keep backups of zone configurations

### Security

1. **CAA Records**: Use CAA records to control certificate issuance
2. **TLSA Records**: Use TLSA records for DANE certificate validation
3. **SSHFP Records**: Use SSHFP records for SSH key verification
4. **Regular Updates**: Keep security records up to date

### Performance

1. **Caching**: Leverage DNS caching for frequently accessed records
2. **TTL Optimization**: Balance TTL values for performance vs. flexibility
3. **Record Count**: Keep zones manageable in size
4. **Resolution**: Use appropriate record types for different use cases

## Integration Examples

### Web3 Applications

```rust
// Resolve IPFS content via DNS
let content_record = dns_system.resolve("example.ipn", Rtype::CONTENT).await?;
if let Some(rrset) = content_record {
    for record in &rrset.records {
        if let Some(obj) = record.as_object() {
            if obj.get("kind") == Some(&json!("ipfs")) {
                if let Some(hash) = obj.get("hash").and_then(|v| v.as_str()) {
                    // Use IPFS hash to fetch content
                    let content = ipfs_client.get(hash).await?;
                }
            }
        }
    }
}
```

### Email Configuration

```rust
// Get MX records for email routing
let mx_records = dns_system.resolve("example.ipn", Rtype::MX).await?;
if let Some(rrset) = mx_records {
    for record in &rrset.records {
        if let Some(obj) = record.as_object() {
            let preference = obj.get("preference").and_then(|v| v.as_u64()).unwrap_or(0);
            let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("");
            // Configure email routing
        }
    }
}
```

### Service Discovery

```rust
// Discover HTTPS services
let https_records = dns_system.resolve("example.ipn", Rtype::HTTPS).await?;
if let Some(rrset) = https_records {
    for record in &rrset.records {
        if let Some(obj) = record.as_object() {
            let target = obj.get("target").and_then(|v| v.as_str()).unwrap_or("");
            let params = obj.get("params");
            // Configure HTTPS client with service parameters
        }
    }
}
```

## Troubleshooting

### Common Issues

1. **Nonce Errors**: Ensure nonce is strictly increasing
2. **Validation Errors**: Check record format and TTL ranges
3. **Conflict Errors**: CNAME/ALIAS records cannot coexist with others
4. **Apex Errors**: CNAME records are not allowed at apex

### Debugging

1. **Check Zone State**: Use zone get command to inspect current state
2. **Validate Records**: Use validation functions to check record formats
3. **Check Logs**: Review transaction logs for detailed error messages
4. **Test Resolution**: Use resolve command to test DNS queries

## Future Enhancements

1. **DNSSEC Signing**: Full DNSSEC support with automated key management
2. **Zone Delegation**: Support for subdomain delegation
3. **Dynamic Updates**: Real-time zone updates with notifications
4. **Advanced Caching**: Multi-level caching with TTL optimization
5. **Zone Templates**: Predefined zone configurations for common use cases
6. **Bulk Operations**: Support for bulk zone updates
7. **Zone Analytics**: Usage statistics and performance metrics
8. **Integration APIs**: REST APIs for third-party integrations
