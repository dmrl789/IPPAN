# IPPAN User Transaction Types

This document describes all 23 canonical transaction types that users can submit to the IPPAN network through the CLI or API.

## Overview

IPPAN supports a comprehensive set of transaction types organized into logical domains:

- **Payments**: Standard transfers, batch payments, and invoices
- **Identity**: Handle registration and management
- **Domains & DNS**: Domain registration and DNS zone management
- **Storage**: File publishing and CDN management
- **Staking**: Validator operations and stake management
- **Faucet**: Bootstrap token distribution
- **Account Management**: Key rotation and controller management
- **Governance**: Protocol voting and proposals
- **Service Payments**: Premium service subscriptions

## Cross-cutting Rules

All transactions follow these common rules:

- **Signatures**: `sig = Sign(ed25519, Blake3(canonical_cbor(tx_without_sig)))`
- **Nonces**: Strictly increasing per state object (handle/domain) to prevent replay
- **Fees**: Base + size-based; reject tx > 16 KB
- **Timestamps**: `|now_us - updated_at_us| ≤ 5s` (where used)
- **Strings**: UTF-8; hard caps (e.g., memo ≤128, TXT chunk ≤255)
- **Addresses/Handles/Domains**: Validated formats and allowed TLDs

---

## 1. Payments

### 1.1 `pay` - Standard Payment

**Purpose**: Send IPN from one account to another (standard transfer).

**Fields**:
- `from` (string): Sender address or handle
- `to` (string): Recipient address or handle
- `amount_ipn` (string): Amount in IPN (decimal string)
- `memo` (string, optional): Memo ≤128 bytes
- `fee` (string): Transaction fee in IPN
- `sig` (string): Transaction signature

**Auth**: `sig(from_sk)`

**Validations**:
- `from` and `to` must be valid addresses or handles
- `amount_ipn` must be positive and ≤ 1 billion IPN
- `memo` must be ≤128 bytes if provided
- `sig` must be valid ed25519 signature

**State effects**: Debit `from`, credit `to`, collect fee

**Example**:
```json
{
  "type": "pay",
  "from": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "to": "@alice.ipn",
  "amount_ipn": "1.25000000",
  "memo": "invoice #42",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli pay --from i1abc... --to @alice.ipn --amount 1.25 --memo "invoice #42" --fee auto --key-file keys/me.json
```

### 1.2 `pay_batch` - Batch Payment

**Purpose**: Multiple transfers atomically from the same payer.

**Fields**:
- `from` (string): Sender address
- `items` (array): Payment items `[{to, amount_ipn, memo?}]`
- `fee` (string): Transaction fee in IPN
- `sig` (string): Transaction signature

**Auth**: `sig(from_sk)`

**Validations**:
- Sum of all amounts + fee ≤ sender balance
- Item count ≤ 100
- Each item must have valid `to` and `amount_ipn`

**Example**:
```json
{
  "type": "pay_batch",
  "from": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "items": [
    {"to": "@a.ipn", "amount_ipn": "0.50000000"},
    {"to": "@b.ipn", "amount_ipn": "0.20000000", "memo": "partial payment"}
  ],
  "fee": "0.00000050",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 1.3 `invoice_create` - Invoice Creation

**Purpose**: Create an on-chain payable request (QR/invoice).

**Fields**:
- `to` (string): Recipient address or handle
- `amount_ipn` (string): Amount in IPN
- `reference` (string, optional): Reference identifier
- `expires_at_us` (u64, optional): Expiration timestamp in microseconds
- `sig` (string): Transaction signature

**Auth**: `sig(to_sk)`

**State effects**: Creates lightweight invoice object; no funds move

**Example**:
```json
{
  "type": "invoice_create",
  "to": "@me.ipn",
  "amount_ipn": "12.34000000",
  "reference": "ORD-9931",
  "expires_at_us": 1756000000000000,
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## 2. Identity: Handles

### 2.1 `handle_register` - Handle Registration

**Purpose**: Buy/register a handle (e.g., `@name.ipn`).

**Fields**:
- `handle` (string): Handle name (e.g., "@desiree.ipn")
- `owner_pk` (string): Owner public key
- `years` (u32): Registration years ≥1
- `fee` (string): Transaction fee (includes registration price)
- `sig` (string): Transaction signature

**Auth**: `sig(purchaser_sk)`

**Validations**:
- Handle must be available
- Handle format: `@[a-z0-9-]{1,63}.ipn`
- Years must be ≥1 and ≤10
- Fee must cover registration price

**State effects**: Create handle record; set expiry

**Example**:
```json
{
  "type": "handle_register",
  "handle": "@desiree.ipn",
  "owner_pk": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "years": 1,
  "fee": "0.50000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli handle-register --handle @desiree.ipn --years 1 --owner-pk keys/me.pub --fee auto --key-file keys/me.json
```

### 2.2 `handle_renew` - Handle Renewal

**Purpose**: Extend handle registration.

**Fields**:
- `handle` (string): Handle name
- `years` (u32): Renewal years
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(owner_sk or controller)`

**Validations**:
- Handle must exist and not be blocked
- Years must be ≥1 and ≤10

**Example**:
```json
{
  "type": "handle_renew",
  "handle": "@desiree.ipn",
  "years": 1,
  "fee": "0.50000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 2.3 `handle_transfer` - Handle Transfer

**Purpose**: Transfer handle ownership to a new key.

**Fields**:
- `handle` (string): Handle name
- `new_owner_pk` (string): New owner public key
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(current_owner_sk)`

**Validations**:
- Handle must exist and not be locked/expired
- New owner public key must be valid

**Example**:
```json
{
  "type": "handle_transfer",
  "handle": "@desiree.ipn",
  "new_owner_pk": "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "fee": "0.01000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 2.4 `handle_update` - Handle Update

**Purpose**: Update handle metadata (addresses, links, avatar, etc.).

**Fields**:
- `handle` (string): Handle name
- `nonce` (u64): Nonce for replay protection
- `ops` (array): Update operations `[{op, path, value?}]`
- `ttl_ms` (u64): TTL in milliseconds
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(owner_sk or controller)`

**Validations**:
- Nonce must be strictly increasing
- Operations must be valid (SET/PATCH/UNSET)
- Size limits on metadata

**Example**:
```json
{
  "type": "handle_update",
  "handle": "@desiree.ipn",
  "nonce": 42,
  "ops": [
    {
      "op": "PATCH",
      "path": "addresses",
      "value": {"ippan": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890"}
    },
    {
      "op": "SET",
      "path": "content",
      "value": "dht:0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
    }
  ],
  "ttl_ms": 3600000,
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli handle-update --handle @desiree.ipn --nonce 42 --ops-file handle_ops.json --ttl-ms 3600000 --fee auto --key-file keys/me.json
```

---

## 3. Domains & DNS

### 3.1 `domain_register` - Domain Registration

**Purpose**: Buy/register a domain (e.g., `example.ipn`).

**Fields**:
- `domain` (string): Domain name (e.g., "example.ipn")
- `owner_pk` (string): Owner public key
- `years` (u32): Registration years
- `plan` (string): Plan type ("standard" or "premium")
- `fee` (string): Transaction fee (includes registration price)
- `sig` (string): Transaction signature

**Auth**: `sig(purchaser_sk)`

**Validations**:
- Domain must be available
- Domain format: `[a-z0-9-]{1,63}(.[a-z0-9-]{1,63})*.ipn`
- Years must be ≥1 and ≤10
- Fee must cover registration price

**Example**:
```json
{
  "type": "domain_register",
  "domain": "example.ipn",
  "owner_pk": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "years": 1,
  "plan": "standard",
  "fee": "8.00000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli domain-register --domain example.ipn --years 1 --plan standard --owner-pk keys/me.pub --fee auto --key-file keys/me.json
```

### 3.2 `domain_renew` - Domain Renewal

**Purpose**: Extend domain registration.

**Fields**:
- `domain` (string): Domain name
- `years` (u32): Renewal years
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(owner_sk or controller)`

**Example**:
```json
{
  "type": "domain_renew",
  "domain": "example.ipn",
  "years": 1,
  "fee": "8.00000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 3.3 `domain_transfer` - Domain Transfer

**Purpose**: Transfer domain to new owner key.

**Fields**:
- `domain` (string): Domain name
- `new_owner_pk` (string): New owner public key
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(current_owner_sk)`

**Example**:
```json
{
  "type": "domain_transfer",
  "domain": "example.ipn",
  "new_owner_pk": "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "fee": "0.05000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 3.4 `zone_update` - DNS Zone Update

**Purpose**: Set DNS-like RRSETs for a domain or subnames.

**Fields**:
- `domain` (string): Domain name
- `nonce` (u64): Nonce for replay protection
- `ops` (array): Zone operations `[{op, name?, rtype?, ttl?, records?}]`
- `updated_at_us` (u64): Update timestamp in microseconds
- `fee_nano` (u64): Transaction fee in nano IPN
- `sig` (string): Transaction signature

**Auth**: `sig(owner_sk or controller)`

**Validations**:
- Nonce must be strictly increasing
- Timestamp skew ≤ 5 seconds
- RR schemas, CNAME/ALIAS exclusivity, TTL bounds
- Size limits on records

**Operations**:
- `UPSERT_RRSET`: Create or update record set
- `DELETE_RRSET`: Delete record set
- `PATCH_RECORDS`: Modify existing records

**Example**:
```json
{
  "type": "zone_update",
  "domain": "example.ipn",
  "nonce": 12,
  "ops": [
    {
      "op": "UPSERT_RRSET",
      "name": "www",
      "rtype": "A",
      "ttl": 300,
      "records": ["93.184.216.34"]
    },
    {
      "op": "UPSERT_RRSET",
      "name": "@",
      "rtype": "ALIAS",
      "ttl": 300,
      "records": ["root.host.net."]
    },
    {
      "op": "UPSERT_RRSET",
      "name": "mail",
      "rtype": "MX",
      "ttl": 300,
      "records": ["10 mail.example.com."]
    }
  ],
  "updated_at_us": 1755327600123456,
  "fee_nano": 100,
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli zone-update --domain example.ipn --nonce 12 --ops-file dns_ops.json --fee-nano auto --key-file keys/me.json
```

---

## 4. Storage / CDN & DHT

### 4.1 `file_publish` - File Publish

**Purpose**: Publish file/content pointer into IPPAN DHT/CDN.

**Fields**:
- `publisher` (string): Publisher address
- `hash_timer` (string): HashTimer content ID
- `size_bytes` (u64): File size in bytes
- `mime` (string): MIME type
- `replicas` (u32): Target number of replicas
- `storage_plan` (string): Storage plan ("free" or "paid")
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(publisher_sk)`

**Validations**:
- Hash format must be valid
- Size limits based on plan
- Replicas must be ≥1 and ≤10

**State effects**: Index entry; triggers storage market

**Example**:
```json
{
  "type": "file_publish",
  "publisher": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "hash_timer": "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "size_bytes": 1048576,
  "mime": "image/png",
  "replicas": 3,
  "storage_plan": "paid",
  "fee": "0.00000100",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli file-publish --publisher i1abc... --file-path ./logo.png --replicas 3 --storage-plan paid --fee auto --key-file keys/me.json
```

### 4.2 `file_update_metadata` - File Metadata Update

**Purpose**: Update metadata (title, tags, license) for published content.

**Fields**:
- `hash_timer` (string): HashTimer content ID
- `ops` (array): Update operations `[{op, path, value?}]`
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(publisher_sk)`

**Example**:
```json
{
  "type": "file_update_metadata",
  "hash_timer": "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "ops": [
    {
      "op": "SET",
      "path": "title",
      "value": "Logo v2"
    },
    {
      "op": "PATCH",
      "path": "tags",
      "value": ["logo", "branding", "v2"]
    }
  ],
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 4.3 `storage_rent_topup` - Storage Rent Topup

**Purpose**: Top up paid storage time/bytes for content.

**Fields**:
- `hash_timer` (string): HashTimer content ID
- `amount_ipn` (string): Amount in IPN
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(publisher_sk or sponsor)`

**Example**:
```json
{
  "type": "storage_rent_topup",
  "hash_timer": "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "amount_ipn": "3.00000000",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 4.4 `pin_request` - Pin Request

**Purpose**: Request additional replicas from storage nodes.

**Fields**:
- `hash_timer` (string): HashTimer content ID
- `replicas` (u32): Number of replicas
- `max_price_ipn` (string): Maximum price in IPN
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(requester_sk)`

**Example**:
```json
{
  "type": "pin_request",
  "hash_timer": "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "replicas": 2,
  "max_price_ipn": "0.10000000",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## 5. Staking / Node Operations

### 5.1 `stake_bond` - Stake Bond

**Purpose**: Lock IPN to become/maintain validator.

**Fields**:
- `validator_pk` (string): Validator public key
- `amount_ipn` (string): Amount in IPN
- `min_lock_days` (u32): Minimum lock period in days
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(account_sk)`

**Validations**:
- Amount ≥ protocol minimum (e.g., 10,000 IPN)
- Lock term must be ≥30 days
- Account must not be jailed

**Example**:
```json
{
  "type": "stake_bond",
  "validator_pk": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "amount_ipn": "10000.00000000",
  "min_lock_days": 30,
  "fee": "0.01000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli stake-bond --validator-pk keys/validator.pub --amount 10000 --min-lock-days 30 --fee auto --key-file keys/me.json
```

### 5.2 `stake_unbond` - Stake Unbond

**Purpose**: Start unbonding (cooldown before withdraw).

**Fields**:
- `validator_pk` (string): Validator public key
- `amount_ipn` (string): Amount in IPN
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(account_sk)`

**Example**:
```json
{
  "type": "stake_unbond",
  "validator_pk": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "amount_ipn": "5000.00000000",
  "fee": "0.01000000",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 5.3 `stake_withdraw` - Stake Withdraw

**Purpose**: Withdraw completed unbonded funds.

**Fields**:
- `validator_pk` (string): Validator public key
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(account_sk)`

**Example**:
```json
{
  "type": "stake_withdraw",
  "validator_pk": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## 6. Faucet / Bootstrap

### 6.1 `faucet_claim` - Faucet Claim

**Purpose**: Claim small bootstrap IPN (gated by proofs/rate).

**Fields**:
- `handle_or_addr` (string): Recipient handle or address
- `uptime_proof` (string): Uptime proof
- `fee` (string): Transaction fee (often zero)
- `sig` (string): Transaction signature

**Auth**: `sig(claimant_sk)`

**Validations**:
- Per-handle cap (e.g., 1 IPN per handle)
- Cooldown period (e.g., 24 hours)
- Proof validation

**Example**:
```json
{
  "type": "faucet_claim",
  "handle_or_addr": "@newuser.ipn",
  "uptime_proof": "proof_data_here",
  "fee": "0",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

**CLI Command**:
```bash
ippan-cli faucet-claim --handle-or-addr @newuser.ipn --uptime-proof proof_data --fee 0 --key-file keys/me.json
```

---

## 7. Account Management

### 7.1 `key_rotate` - Key Rotation

**Purpose**: Rotate the spending key for an address.

**Fields**:
- `address` (string): Account address
- `new_owner_pk` (string): New owner public key
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(current_owner_sk)`

**Example**:
```json
{
  "type": "key_rotate",
  "address": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "new_owner_pk": "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

### 7.2 `set_controllers` - Set Controllers

**Purpose**: Add/remove controller keys (delegation) for handle/domain.

**Fields**:
- `target_type` (string): Target type ("handle" or "domain")
- `target_id` (string): Target ID
- `controllers` (array): Controller public keys
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(owner_sk)`

**Example**:
```json
{
  "type": "set_controllers",
  "target_type": "domain",
  "target_id": "example.ipn",
  "controllers": [
    "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  ],
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## 8. Governance / Protocol

### 8.1 `gov_vote` - Governance Vote

**Purpose**: Vote on active proposal.

**Fields**:
- `proposal_id` (string): Proposal ID
- `choice` (string): Vote choice ("yes", "no", "abstain")
- `stake_weight` (string, optional): Stake weight
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(voter_sk)`

**Example**:
```json
{
  "type": "gov_vote",
  "proposal_id": "P-2025-08-17-01",
  "choice": "yes",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## 9. Service Payments

### 9.1 `service_pay` - Service Payment

**Purpose**: Pay a named on-chain service (e.g., premium domain, CDN plan).

**Fields**:
- `service_id` (string): Service ID
- `plan` (string): Plan type
- `amount_ipn` (string): Amount in IPN
- `period` (string, optional): Period
- `fee` (string): Transaction fee
- `sig` (string): Transaction signature

**Auth**: `sig(payer_sk)`

**Example**:
```json
{
  "type": "service_pay",
  "service_id": "cdn.tier2",
  "plan": "monthly",
  "amount_ipn": "2.50000000",
  "period": "monthly",
  "fee": "0.00000010",
  "sig": "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
```

---

## Fee Structure (PRD-aligned)

### Canonical Rule
**Fee = 1% of the transferred amount** for **every on-chain transaction**.
**Destination:** 100% of fees flow to the **Keyless Global Fund** (weekly auto-distribution).

### Exact Formula
Let `amount` be in the smallest unit (1 IPN = 1e8 units).

```
fee_units = max(1, floor(amount * 0.01))   // at least 1 unit (dust guard)
payer_pays = amount + fee_units
```

**Rationale:** PRD mandates a flat 1% network fee; a 1-unit minimum prevents zero-fee dust.

### What We Are **Not** Doing
* No base/size/memo surcharges (not in PRD).
* Keep memos payload-agnostic; cost is purely the 1% fee.

### Fee Examples
- **1.00 IPN payment**: Fee = 0.01 IPN (1%)
- **0.10 IPN payment**: Fee = 0.001 IPN (1%)
- **0.001 IPN payment**: Fee = 0.00001 IPN (1%)
- **0.00000001 IPN payment**: Fee = 1 unit (dust guard minimum)

### M2M (Machine-to-Machine) Payments
M2M in IPPAN supports **high-frequency micro-transfers** without spamming L1 through **ephemeral channels + periodic settlements**, while keeping the **1% rule** intact at settlement.

#### Channel Lifecycle
1. **Open (on-chain)**: 1% fee on the deposit amount
2. **Stream (off-chain updates)**: No fees per off-chain update
3. **Settle (on-chain)**: 1% fee on the net settled amount
4. **Close (on-chain)**: 1% fee on the final net settlement

#### Fee Invariants
* **Never more than 1% total on value that reaches L1.**
* Opening fee: 1% of deposit.
* Settlement fee: 1% of net amount moved from payer→payee on chain.
* Off-chain updates: 0%.
* Result: tiny, rapid M2M flows with very few L1 touches, while the Global Fund still accrues 1% on actual value that settles.

### Automatic Fee Calculation
Use `--fee auto` in CLI commands to automatically calculate the 1% fee based on transaction amount.

---

## Validation Rules

### Address Format
- Must start with "i1"
- Length: 3-100 characters
- Valid base58 encoding

### Handle Format
- Must start with "@"
- Must end with ".ipn"
- Name: 1-63 alphanumeric characters or hyphens
- Cannot start or end with hyphen

### Domain Format
- Must end with ".ipn"
- Total length ≤ 253 characters
- Labels: 1-63 alphanumeric characters or hyphens
- Cannot start or end with hyphen

### Amount Format
- Decimal string with up to 8 decimal places
- Must be positive
- Maximum: 1 billion IPN

### Signature Format
- Must start with "ed25519:"
- 64 bytes (128 hex characters)
- Valid hex encoding

---

## Error Handling

All transactions return structured error responses:

```json
{
  "error": "validation_error",
  "message": "Invalid handle format",
  "field": "handle",
  "code": "INVALID_FORMAT"
}
```

Common error codes:
- `INVALID_FORMAT`: Malformed input
- `INSUFFICIENT_BALANCE`: Not enough funds
- `INVALID_SIGNATURE`: Signature verification failed
- `NONCE_TOO_LOW`: Nonce must be strictly increasing
- `TIMESTAMP_SKEW`: Timestamp too far from current time
- `SIZE_LIMIT_EXCEEDED`: Transaction too large
- `NOT_AUTHORIZED`: Insufficient permissions

---

## Best Practices

1. **Always use automatic fee calculation** (`--fee auto`) unless you have specific requirements
2. **Validate inputs** before submitting transactions
3. **Keep nonces strictly increasing** for each account/handle/domain
4. **Use appropriate TTLs** for DNS records (300-3600 seconds recommended)
5. **Backup private keys** securely
6. **Monitor transaction status** after submission
7. **Use batch operations** for multiple related changes
8. **Test on testnet** before mainnet deployment

---

## Integration Examples

### JavaScript/TypeScript
```typescript
import { IppanClient } from '@ippan/client';

const client = new IppanClient('https://api.ippan.network');

// Send payment
const tx = await client.createPayment({
  from: 'i1abc...',
  to: '@alice.ipn',
  amount: '1.25',
  memo: 'invoice #42'
});

await client.submitTransaction(tx);
```

### Python
```python
from ippan import IppanClient

client = IppanClient('https://api.ippan.network')

# Register domain
tx = client.create_domain_registration(
    domain='example.ipn',
    years=1,
    plan='standard'
)

client.submit_transaction(tx)
```

### Rust
```rust
use ippan_client::IppanClient;

let client = IppanClient::new("https://api.ippan.network");

// Update DNS zone
let tx = client.create_zone_update(
    domain: "example.ipn",
    nonce: 12,
    ops: vec![
        ZoneOp::upsert_rrset("www", "A", 300, vec!["93.184.216.34"])
    ]
);

client.submit_transaction(tx).await?;
```

This comprehensive transaction system provides IPPAN with a complete set of user-facing operations for building decentralized applications, identity management, content distribution, and governance.
