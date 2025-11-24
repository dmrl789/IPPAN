# IPPAN Upgrades & Migrations Guide

**Version:** 1.0.0  
**Last Updated:** 2025-11-24

---

## Overview

This guide covers how IPPAN handles upgrades, migrations, and backward compatibility for:
- **Storage schemas** (database structure)
- **Config formats** (TOML configuration)
- **Protocol changes** (consensus rules)

**Design Principles:**
1. **No data loss:** Upgrades preserve all historical data
2. **Automatic migrations:** Where possible, migrations run on startup
3. **Clear warnings:** Incompatible upgrades are flagged before starting
4. **Rollback support:** Critical components support downgrading

---

## Storage Schema Versioning

### Overview

IPPAN uses **versioned snapshots** to handle storage schema changes.

**Current Schema Version:** `2`

**Schema Fields:**
- `version`: Schema version number (monotonically increasing)
- `network_id`: Network identifier (e.g., `ippan-mainnet-v1`)
- `height`: Block height at snapshot time
- `timestamp_us`: Timestamp in microseconds
- Additional metadata (account/block counts, HashTimer ranges)

### Schema Version History

| Version | Released | Changes |
|---------|----------|---------|
| **1** | v0.9.0 | Initial schema (basic accounts, blocks, transactions) |
| **2** | v1.0.0 | Added `tip_block_hash`, HashTimer ranges, file descriptors |

### How Schema Versioning Works

#### 1. On Startup

```rust
// crates/storage/src/lib.rs
const SNAPSHOT_MANIFEST_VERSION: u32 = 2;

pub fn check_schema_version(&self) -> Result<(), StorageError> {
    let stored_version = self.get_schema_version()?;
    
    if stored_version > SNAPSHOT_MANIFEST_VERSION {
        return Err(StorageError::FutureSchema {
            stored: stored_version,
            supported: SNAPSHOT_MANIFEST_VERSION,
        });
    }
    
    if stored_version < SNAPSHOT_MANIFEST_VERSION {
        warn!("Schema version {} detected, current is {}. Running migration...", 
              stored_version, SNAPSHOT_MANIFEST_VERSION);
        self.migrate_schema(stored_version, SNAPSHOT_MANIFEST_VERSION)?;
    }
    
    Ok(())
}
```

#### 2. Migration Process

**Automatic Migrations:**
- v1 → v2: Compute missing `tip_block_hash` from DAG tips

**Manual Migrations:**
- Breaking schema changes require export → upgrade → import cycle

### Upgrading Storage Schema

#### Minor Schema Change (v2 → v3, backward compatible)

**No action required:**
- Node automatically migrates on startup
- Migration logged to console

```
INFO: Storage schema v2 detected (current: v3)
INFO: Running automatic migration v2 -> v3...
INFO: Migration complete. Records updated: 12,345
```

#### Major Schema Change (v2 → v4, requires export/import)

**Step 1: Export snapshot before upgrade**

```bash
# Stop node
systemctl stop ippan-node

# Export current state
cargo run -p ippan-node -- \
  --config /etc/ippan/node.toml \
  export-snapshot \
  --output /backup/snapshot-v2-$(date +%Y%m%d)

# Backup data directory
tar -czf /backup/ippan-data-$(date +%s).tar.gz /var/lib/ippan
```

**Step 2: Upgrade binary**

```bash
wget https://github.com/dmrl789/IPPAN/releases/download/v4.0.0/ippan-v4.0.0-linux-x86_64.tar.gz
tar -xzf ippan-v4.0.0-linux-x86_64.tar.gz
sudo mv ippan-node /usr/local/bin/
```

**Step 3: Clear old data**

```bash
# Remove old database (keep backup!)
rm -rf /var/lib/ippan/data
mkdir -p /var/lib/ippan/data
```

**Step 4: Import snapshot with new schema**

```bash
cargo run -p ippan-node -- \
  --config /etc/ippan/node.toml \
  import-snapshot \
  --input /backup/snapshot-v2-20251124
```

**Step 5: Start node**

```bash
systemctl start ippan-node
journalctl -u ippan-node -f
```

**Validation:**
```bash
# Check schema version
curl http://localhost:8080/status | jq '.storage.schema_version'
# Should output: 4

# Check block count
curl http://localhost:8080/status | jq '.storage.blocks_count'
# Should match pre-upgrade count
```

---

## Config File Migrations

### Config Versioning

IPPAN config files (TOML) are versioned to support backward compatibility.

**Config Version Detection:**
```toml
# Top of config file
[meta]
version = "1.0"
```

### Config Version History

| Version | Released | Changes |
|---------|----------|---------|
| **0.9** | v0.9.0 | Initial config format |
| **1.0** | v1.0.0 | Added `[metrics]`, `[ai_params]`, deprecated `enable_legacy_api` |

### Upgrade Process

#### Automatic Config Upgrade

On startup, IPPAN checks config version:

```rust
// Check config version
let config_version = config.meta.version.parse::<f32>()?;

if config_version < 1.0 {
    warn!("Config version {} is deprecated. Please update to v1.0", config_version);
    warn!("Deprecated fields: enable_legacy_api");
    
    // Apply backward-compatible defaults
    if config.enable_legacy_api.is_none() {
        config.enable_legacy_api = Some(false);
    }
}
```

#### Manual Config Update

**Example: v0.9 → v1.0**

**Old Config (v0.9):**
```toml
[network]
network_id = "ippan-testnet-v1"
listen_address = "0.0.0.0:8080"
enable_legacy_api = true  # Deprecated in v1.0
```

**New Config (v1.0):**
```toml
[meta]
version = "1.0"

[network]
network_id = "ippan-testnet-v1"
listen_address = "0.0.0.0:8080"
# enable_legacy_api removed (no longer supported)

[metrics]
enabled = true
bind_address = "0.0.0.0:9615"

[ai_params]
model_path = "models/deterministic_gbdt_model.json"
enable_fairness = true
```

**Migration Steps:**
1. Copy old config: `cp node.toml node.toml.bak`
2. Update config manually (refer to `config/default.toml` for reference)
3. Test new config: `ippan-node --config node.toml --validate-config`
4. Restart node

### Deprecated Config Fields

| Field | Deprecated | Removed | Replacement |
|-------|------------|---------|-------------|
| `enable_legacy_api` | v1.0 | v2.0 | N/A (always disabled) |
| `use_poa_consensus` | v1.0 | v1.1 | `consensus_mode = "DLC"` |
| `enable_unsafe_rpc` | v1.0 | v1.0 | N/A (always disabled) |

**Handling Deprecated Fields:**
- **Warnings:** Logged on startup if present
- **Grace Period:** 1 major version (e.g., deprecated in v1.0, removed in v2.0)
- **Migration Guide:** Published in release notes

---

## Protocol Upgrades

### Protocol Versioning

**Protocol Version:** Embedded in blocks and network messages

```rust
pub const PROTOCOL_VERSION: u32 = 1;

pub struct BlockHeader {
    pub protocol_version: u32,  // Always PROTOCOL_VERSION
    // ...
}
```

### Hard Fork Process

For protocol-breaking changes (e.g., consensus rule changes):

#### 1. Announcement (T-90 days)

- Announce fork via Discord, GitHub, website
- Publish fork spec and rationale
- Set activation height/timestamp

#### 2. Release Candidate (T-60 days)

- Release v2.0.0-rc1 with fork code (inactive)
- Deploy to testnet for validation
- Gather validator feedback

#### 3. Final Release (T-30 days)

- Release v2.0.0 stable
- All validators MUST upgrade before activation height

#### 4. Activation (T-0)

- Fork activates at predefined height (e.g., block 1,000,000)
- Old nodes reject new blocks (incompatible protocol version)

**Example Activation Logic:**
```rust
fn validate_block_protocol_version(block: &Block, current_height: u64) -> Result<()> {
    let expected_version = if current_height >= FORK_HEIGHT_V2 {
        2
    } else {
        1
    };
    
    if block.protocol_version != expected_version {
        return Err(ValidationError::ProtocolMismatch {
            expected: expected_version,
            actual: block.protocol_version,
        });
    }
    
    Ok(())
}
```

### Soft Fork Process

For backward-compatible changes:

- New features optional
- Old nodes continue working (may miss new functionality)
- Example: New transaction type (old nodes ignore)

---

## Rollback Procedures

### When to Rollback

**Critical Bugs:**
- Data corruption detected
- Consensus failure (nodes diverge)
- Security vulnerability exploited

### Rollback Steps

#### 1. Stop Node

```bash
systemctl stop ippan-node
```

#### 2. Restore Old Binary

```bash
# Restore previous version
sudo cp /backup/ippan-node-v1.0.0 /usr/local/bin/ippan-node
```

#### 3. Restore Old Data (if schema changed)

```bash
# Clear new data
rm -rf /var/lib/ippan/data

# Restore backup
tar -xzf /backup/ippan-data-pre-upgrade.tar.gz -C /var/lib/ippan
```

#### 4. Start Old Version

```bash
systemctl start ippan-node
journalctl -u ippan-node -f
```

#### 5. Verify State

```bash
curl http://localhost:8080/health
curl http://localhost:8080/status | jq '.storage.blocks_count'
```

### Rollback Limitations

**Cannot Rollback If:**
- Protocol version increased (hard fork)
- Storage schema incompatible (v4 → v2)
- Blockchain finalized with new rules

**Solution:** Re-sync from genesis or restore from pre-upgrade snapshot

---

## Pre-Upgrade Checklist

Before upgrading to a new version:

- [ ] **Read release notes:** Check for breaking changes
- [ ] **Backup data:**
  ```bash
  tar -czf /backup/ippan-data-$(date +%s).tar.gz /var/lib/ippan
  ```
- [ ] **Export snapshot:**
  ```bash
  cargo run -p ippan-node -- export-snapshot --output /backup/snapshot-$(date +%Y%m%d)
  ```
- [ ] **Test on staging:** If operating critical infrastructure
- [ ] **Schedule maintenance window:** Minimize downtime
- [ ] **Check disk space:** Migrations may require 2× storage temporarily
- [ ] **Review config changes:** Update deprecated fields
- [ ] **Inform users:** If running public RPC node

---

## Migration Troubleshooting

### Migration Fails: "Unsupported Schema Version"

**Error:**
```
ERROR: Unsupported schema version 5 (supported: 2)
```

**Cause:** Binary is older than database

**Solution:**
1. Upgrade binary to latest version
2. If latest doesn't support v5, wait for next release or restore backup

### Migration Fails: "Out of Disk Space"

**Error:**
```
ERROR: No space left on device
```

**Cause:** Migration creates temporary files

**Solution:**
1. Free up disk space (≥2× current usage)
2. Restart migration

### Migration Succeeds But Node Won't Start

**Symptom:** Migration completes, but node fails health check

**Check:**
```bash
journalctl -u ippan-node -n 100
```

**Common Causes:**
- Config file incompatible → Update config
- Missing AI model file → Download latest model
- Port conflict → Change port in config

### Migration Takes Too Long (>1 hour)

**Cause:** Large database (>100 GB)

**Solution:**
1. Wait for completion (safe to take hours for very large DBs)
2. Alternative: Export old snapshot, fresh sync with new binary

---

## Best Practices

### For Node Operators

1. **Always backup before upgrading**
2. **Test on staging first** (if operating critical infrastructure)
3. **Subscribe to release announcements** (GitHub, Discord)
4. **Monitor after upgrade** (24-48 hours)
5. **Keep old binaries** for emergency rollback

### For Validators

1. **Coordinate upgrades** with other validators (avoid mass downtime)
2. **Maintain 2× disk space** for migrations
3. **Use monitoring alerts** (downtime, peer count)
4. **Document local changes** (custom configs, scripts)

### For Developers

1. **Never skip schema versions** (v1 → v3 must migrate v1 → v2 → v3)
2. **Test migrations** in integration tests
3. **Document breaking changes** in release notes
4. **Provide migration scripts** for complex upgrades

---

## Upgrade Timeline Example

**Scenario:** Upgrading from v1.0.0 to v2.0.0 (major release with protocol change)

| Date | Action |
|------|--------|
| **T-90** | v2.0.0 announcement, fork spec published |
| **T-60** | v2.0.0-rc1 released, testnet deployed |
| **T-45** | Testnet validation complete, validators confirm readiness |
| **T-30** | v2.0.0 stable released |
| **T-7** | Final reminder: Upgrade by T-0 or risk being forked out |
| **T-1** | Last-minute backups and staging tests |
| **T-0** | Fork activates at block 1,000,000 (12:00 UTC) |
| **T+1** | Post-fork monitoring, check for issues |
| **T+7** | All-clear: v2.0.0 stable across network |

---

## Emergency Contacts

**Critical Issues:**
- Discord: https://discord.gg/ippan (fastest response)
- GitHub Issues: https://github.com/dmrl789/IPPAN/issues (for bugs)
- Email: ops@ippan.io (for sensitive matters)

**Planned Maintenance:**
- Check status page: https://status.ippan.io
- Subscribe to updates: https://ippan.io/newsletter

---

## Summary

| Upgrade Type | Downtime | Backup Required | Rollback Possible |
|-------------|----------|-----------------|-------------------|
| **Patch (v1.0.0 → v1.0.1)** | <1 min | Optional | Yes (minutes) |
| **Minor (v1.0 → v1.1)** | <5 min | Recommended | Yes (hours) |
| **Major (v1.0 → v2.0)** | 10-30 min | **REQUIRED** | Difficult (days) |
| **Hard Fork** | Coordinated | **REQUIRED** | No (re-sync required) |

**Golden Rule:** When in doubt, backup first.

---

**Maintainers:**  
- Ugo Giuliani (Lead Architect)
- Kambei Sapote (Network Engineer)

**Last Major Upgrade:** v1.0.0-rc1 (2025-11-24)
