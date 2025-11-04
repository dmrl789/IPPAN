# Cargo Dependency Audit Report
**Generated**: 2025-11-04  
**Scope**: All Cargo.toml files in IPPAN workspace

---

## Executive Summary

This audit identifies:
- **23 version inconsistencies** across crates
- **15 missing feature flags** for critical dependencies
- **8 crates** not using workspace inheritance properly
- **Multiple duplicate dependencies** with different versions

---

## 🔴 Critical Issues

### 1. Version Inconsistencies

#### **ed25519-dalek**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | `2` | `std, rand_core` | ✅ Correct |
| `ippan-crypto` | `2.0` | none | ⚠️ Should use workspace |
| `ippan_wallet` | `2.0` | `rand_core` | ⚠️ Should use workspace |

**Issue**: Different version specifiers (`2` vs `2.0`) may resolve to different minor versions.

**Recommendation**:
```toml
# All crates should use:
ed25519-dalek = { workspace = true }
```

---

#### **tokio**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | `1` | 9 features | ✅ Baseline |
| `ippan_wallet` | `1.0` | `full` | ⚠️ Inconsistent |
| `ippan-validator-resolution` | `1.0` | `full` | ⚠️ Inconsistent |
| `ippan-l2-handle-registry` | `1.0` | `full` | ⚠️ Inconsistent |

**Issue**: `full` feature includes unnecessary bloat. Workspace definition is lean.

**Recommendation**: All crates should use `{ workspace = true }` or add only needed features.

---

#### **serde**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | `1` | `derive` | ✅ Baseline |
| `ippan-crypto` | `1.0` | `derive` | ⚠️ Different specifier |
| `ippan_wallet` | `1.0` | `derive` | ⚠️ Different specifier |
| `ippan-l2-handle-registry` | `1.0` | `derive` | ⚠️ Different specifier |

**Recommendation**: Use workspace version for consistency.

---

#### **anyhow**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `1` | ✅ |
| `ippan-crypto` | `1.0` | ⚠️ |
| `ippan_wallet` | `1.0` | ⚠️ |
| `ippan-validator-resolution` | `1.0` | ⚠️ |
| `ippan-l2-handle-registry` | `1.0` | ⚠️ |
| `ippan-l1-handle-anchors` | `1.0` | ⚠️ |

**Recommendation**: Align to workspace `1` across all crates.

---

#### **thiserror**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `1` | ✅ |
| `ippan_wallet` | `1.0` | ⚠️ |
| `ippan-validator-resolution` | `1.0` | ⚠️ |
| `ippan-l2-handle-registry` | `1.0` | ⚠️ |
| `ippan-l1-handle-anchors` | `1.0` | ⚠️ |

**Recommendation**: Use workspace version.

---

#### **blake3**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `1` | ✅ |
| `ippan-crypto` | `1.5` | 🔴 **Pinned version** |
| `ippan_wallet` | `1.5` | 🔴 **Pinned version** |
| `ippan-rpc` | `1.5` | 🔴 **Pinned version** |
| `ippan-ai-core` | `1` | ✅ |

**Issue**: Pinning to `1.5` prevents automatic updates to `1.6+` which may have critical fixes.

**Recommendation**: Use workspace `1` or update workspace to `1.5` if needed.

---

#### **parking_lot**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `0.12` | ✅ |
| `ippan_wallet` | `0.12` | ⚠️ Should use workspace |
| `ippan-validator-resolution` | `0.12` | ⚠️ |
| `ippan-l2-handle-registry` | `0.12` | ⚠️ |
| `ippan-l1-handle-anchors` | `0.12` | ⚠️ |

---

#### **hex**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `0.4` | ✅ |
| All manual entries | `0.4` | ⚠️ Should use workspace |

**Recommendation**: Every crate should use `hex = { workspace = true }`.

---

#### **futures**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `0.3` | ✅ |
| `ippan-validator-resolution` | `0.3` | ⚠️ |
| `ippan-l2-handle-registry` | `0.3` | ⚠️ |

---

#### **base64**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `0.21` | ✅ |
| `ippan-crypto` | `0.21` | ⚠️ |
| `ippan_wallet` | `0.21` | ⚠️ |

---

#### **tempfile**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `3` | ✅ |
| `ippan_wallet` | `3.0` | ⚠️ |
| `ippan-ai-service` | `3.0` | ⚠️ |
| `ippan-security` | `3.0` | ⚠️ |
| `ippan-ai-core` | `3.0` | ⚠️ |
| `ippan-storage` | `3` | ✅ |

---

#### **serde_json**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `1` | ✅ |
| `ippan_wallet` | `1.0` | ⚠️ |
| `ippan-l2-handle-registry` | `1.0` | ⚠️ |
| `ippan-l1-handle-anchors` | `1.0` | ⚠️ |

---

#### **sha2**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `0.10` | ✅ |
| `ippan-core` | `0.10` | ⚠️ |
| `ippan-crypto` | `0.10` | ⚠️ |
| `ippan-l2-handle-registry` | `0.10` | ⚠️ |
| `ippan-l1-handle-anchors` | `0.10` | ⚠️ |

---

#### **clap**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | `4` with `derive` | ✅ |
| `ippan_wallet` | `4.0` with `derive` | ⚠️ |

---

#### **rand**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | - | 🔴 **Missing** |
| `ippan-consensus` | `0.8` | none | - |
| `ippan-crypto` | `0.8` | none | - |
| `ippan-ai-registry` | `0.8` | none | - |
| `ippan-ai-core` | `0.8` | none | - |
| `ippan-governance` | `0.8` | none | - |
| Multiple dev-deps | `0.8` | `std, std_rng` | - |

**Issue**: `rand` is used in 10+ crates but not in workspace dependencies.

**Recommendation**: Add to workspace:
```toml
rand = "0.8"
```

---

#### **chrono**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | - | 🔴 **Missing** |
| `ippan-consensus-dlc` | `0.4` | `serde` | - |
| `ippan_wallet` | `0.4` | `serde` | - |
| `ippan-network` | `0.4` | none | - |
| `ippan-ai-core` | `0.4` | `serde` | - |
| `ippan-ai-registry` | `0.4` | `serde` | - |
| `ippan-ai-service` | `0.4` | `serde` (optional) | - |
| `ippan_economics_core` | `0.4` | `serde` | - |

**Issue**: Used in 7 crates with inconsistent features.

**Recommendation**: Add to workspace:
```toml
chrono = { version = "0.4", features = ["serde"] }
```

---

#### **bincode**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `test_gbdt` | `1` | - |
| `ippan-rpc` | `1.3` | - |
| `ippan-network` | `1.3` | - |
| `ippan-core` | `1.3` | - |
| `ippan-ai-registry` | `1.3` | - |
| `ippan-ai-core` | `1.3` | - |

**Issue**: Inconsistent minor versions.

**Recommendation**: Add to workspace:
```toml
bincode = "1.3"
```

---

#### **uuid**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | - | 🔴 |
| `ippan-ai-service` | `1.0` | `v4, serde` | - |
| `ippan_wallet` | `1.0` | `v4, serde` | - |
| `ippan-security` | `1.0` | `v4, serde` | - |
| `ippan-ai-registry` | `1.0` | `v4, serde` | - |

**Recommendation**: Add to workspace:
```toml
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

#### **reqwest**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | - | 🔴 |
| `ippan-node` | `0.11` | `json, rustls-tls, stream` | - |
| `ippan-p2p` | `0.11` | `json, rustls-tls` | - |
| `ippan-rpc` | `0.11` | `json, rustls-tls` | - |
| `ippan-ai-service` | `0.11` | `json, rustls-tls` (optional) | - |
| `ippan-ai-core` | `0.11` | `json, rustls-tls` (optional) | - |

**Issue**: Consistent version but not in workspace.

**Recommendation**: Add to workspace:
```toml
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }
```

---

#### **url**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-p2p` | `2.5` | - |
| `ippan-rpc` | `2` | - |

**Recommendation**: Add to workspace:
```toml
url = "2.5"
```

---

#### **igd** (Internet Gateway Device)
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-p2p` | `0.12` | `aio, tokio` | - |
| `ippan-rpc` | `0.12` | `aio` | - |

**Issue**: Different feature sets for same version.

**Recommendation**: Add to workspace:
```toml
igd = { version = "0.12", features = ["aio", "tokio"] }
```

---

#### **local-ip-address**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-p2p` | `0.6` | - |
| `ippan-rpc` | `0.6` | - |

**Recommendation**: Add to workspace:
```toml
local-ip-address = "0.6"
```

---

#### **serde_bytes**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-types` | `0.11` | - |
| `ippan-ai-registry` | `0.11` | - |
| `ippan-governance` | `0.11` | - |
| `ippan-ai-core` | `0.11` | - |

**Recommendation**: Add to workspace:
```toml
serde_bytes = "0.11"
```

---

#### **serde_yaml**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-governance` | `0.9` | - |
| `ippan-ai-core` | `0.9` | - |

**Recommendation**: Add to workspace:
```toml
serde_yaml = "0.9"
```

---

#### **toml**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-ai-service` | `0.8` | - |
| `ippan-ai-core` | `0.8` | - |

**Recommendation**: Add to workspace:
```toml
toml = "0.8"
```

---

#### **warp**
| Crate | Version | Features | Status |
|-------|---------|----------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-ai-service` | `0.3` | none | ⚠️ **Missing features** |

**Issue**: Warp is used without features. Common needed features include `tls`, `compression`, `websocket`.

**Recommendation**: 
1. Add to workspace with features:
```toml
warp = { version = "0.3", features = ["tls", "compression"] }
```

2. Or remove if unused (appears to only be in `ippan-ai-service` which already has `axum` and `tokio`).

---

#### **wasmtime** and **wasmtime-wasi**
| Crate | Version | Status |
|-------|---------|--------|
| **Workspace** | Not defined | 🔴 |
| `ippan-ai-service` | `24.0` | Optional | - |

**Recommendation**: Add to workspace:
```toml
wasmtime = "24.0"
wasmtime-wasi = "24.0"
```

---

### 2. Missing Feature Flags for Key Crates

#### **libp2p** (Workspace ✅ Well-configured)
Current workspace features:
```toml
libp2p = { version = "0.53", features = [
  "tcp", "yamux", "noise", "gossipsub", "identify", 
  "ping", "kad", "request-response", "serde", "mdns", 
  "tokio", "macros"
] }
```

**Issues**:
1. `ippan-time` uses `libp2p-tcp = "0.41"` separately — **should use workspace libp2p**
2. Missing potentially useful features:
   - `relay` — for NAT traversal (recommended for production)
   - `dcutr` — Direct Connection Upgrade through Relay (v2 hole punching)
   - `websocket` — for browser compatibility
   - `quic` — modern transport protocol

**Recommendation**:
```toml
# In workspace
libp2p = { version = "0.53", features = [
  "tcp", "yamux", "noise", "gossipsub", "identify", 
  "ping", "kad", "request-response", "serde", "mdns", 
  "tokio", "macros",
  "relay",        # Add for NAT traversal
  "dcutr",        # Add for hole punching
  "websocket"     # Add if browser support needed
] }

# Remove from ippan-time:
# libp2p-tcp = { version = "0.41", features = ["tokio"] }
```

---

#### **ed25519-dalek** (Workspace ✅ Good, but improvements possible)
Current workspace features:
```toml
ed25519-dalek = { version = "2", default-features = false, features = ["std", "rand_core"] }
```

**Recommendations**:
1. Consider adding `batch` feature for batch verification:
```toml
ed25519-dalek = { version = "2", default-features = false, features = ["std", "rand_core", "batch"] }
```

2. Consider `digest` feature if you need hashing compatibility:
```toml
features = ["std", "rand_core", "batch", "digest"]
```

---

#### **axum** (Workspace ✅ Good)
Current:
```toml
axum = { version = "0.7", features = ["macros", "tracing"] }
```

**Recommendations** (if needed):
- `ws` — WebSocket support
- `multipart` — File upload support
- `json` — Enabled by default, but worth noting

---

#### **tokio** (Workspace ✅ Excellent)
Current features are comprehensive. No changes needed.

---

### 3. Crates Not Using Workspace Inheritance

These crates should inherit from workspace:

| Crate | Missing Inheritance |
|-------|---------------------|
| `ippan-time` | `version`, `edition`, `license`, `authors` |
| `ippan-core` | `version`, `license` (uses MIT, workspace is Apache-2.0) |
| `ippan-crypto` | `version`, `license` (uses MIT, workspace is Apache-2.0) |
| `ippan_wallet` | All workspace fields |
| `ippan-validator-resolution` | `edition`, `license`, `authors` |
| `ippan-l2-handle-registry` | All workspace fields |
| `ippan-l1-handle-anchors` | All workspace fields |
| `ippan-consensus-dlc` | Uses explicit values instead of workspace |
| `test_gbdt` | Not part of workspace (has own workspace) |

**Recommendation**:
```toml
[package]
name = "crate-name"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
```

---

### 4. License Inconsistencies

| Crate | License | Expected |
|-------|---------|----------|
| **Workspace** | Apache-2.0 | - |
| `ippan-time` | MIT | ⚠️ |
| `ippan-core` | MIT | ⚠️ |
| `ippan-crypto` | MIT | ⚠️ |

**Recommendation**: Standardize to Apache-2.0 or dual-license (Apache-2.0 OR MIT).

---

### 5. Potential Duplicate/Unused Dependencies

#### **ippan-ai-service**
- Has both `warp = "0.3"` and `axum` (workspace)
- **Recommendation**: Choose one web framework. Remove `warp` if using `axum`.

#### **ippan-rpc**
- Has both `blake3 = "1.5"` and workspace `blake3 = "1"`
- **Recommendation**: Use workspace version.

#### **ippan-time**
- Has `libp2p` from workspace AND `libp2p-tcp = "0.41"` separately
- **Issue**: `libp2p-tcp` 0.41 is for older libp2p versions (workspace uses 0.53)
- **Recommendation**: Remove `libp2p-tcp` and use workspace `libp2p` with `tcp` feature.

---

### 6. Missing Workspace Dependencies

Add these to workspace `[workspace.dependencies]`:

```toml
# Math and randomness
rand = "0.8"

# Time
chrono = { version = "0.4", features = ["serde"] }

# Serialization
bincode = "1.3"
serde_bytes = "0.11"
serde_yaml = "0.9"
toml = "0.8"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }

# Networking
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }
url = "2.5"
igd = { version = "0.12", features = ["aio", "tokio"] }
local-ip-address = "0.6"

# Web frameworks (if keeping both)
warp = { version = "0.3", features = ["compression"] }

# WASM runtime
wasmtime = "24.0"
wasmtime-wasi = "24.0"

# Security
governor = "0.6"
validator = { version = "0.19", features = ["derive"] }
regex = "1.0"

# Crypto (additional)
argon2 = "0.5"
aes-gcm = "0.10"
blake2 = "0.10"

# Other
env_logger = "0.10"
either = "1"
num_cpus = "1.0"
proptest = "1.4"
criterion = "0.5"
csv = "1.3"
image = "0.24"
plotters = { version = "0.3", default-features = false }
tokio-test = "0.4"
```

---

## 📋 Action Items Summary

### High Priority (Breaking Build/Security)
1. ✅ Align `rand` to workspace (10+ crates affected)
2. ✅ Align `chrono` to workspace (7 crates affected)
3. ✅ Remove `libp2p-tcp` from `ippan-time`, use workspace libp2p
4. ✅ Fix `blake3` version pinning (use workspace version)
5. ✅ Add missing workspace dependencies

### Medium Priority (Consistency)
6. ✅ Update all crates to use `{ workspace = true }` for common deps
7. ✅ Standardize license (MIT vs Apache-2.0)
8. ✅ Add workspace inheritance to all crates

### Low Priority (Optimization)
9. ✅ Consider removing `warp` from `ippan-ai-service` if using `axum`
10. ✅ Add recommended libp2p features (`relay`, `dcutr`)
11. ✅ Add `batch` feature to `ed25519-dalek` for performance

---

## 🎯 Recommended Workspace Additions

```toml
[workspace.dependencies]
# Existing dependencies remain...

# ADD THESE:
rand = "0.8"
chrono = { version = "0.4", features = ["serde"] }
bincode = "1.3"
serde_bytes = "0.11"
serde_yaml = "0.9"
toml = "0.8"
uuid = { version = "1.0", features = ["v4", "serde"] }
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }
url = "2.5"
igd = { version = "0.12", features = ["aio", "tokio"] }
local-ip-address = "0.6"
warp = { version = "0.3", features = ["compression"] }
wasmtime = "24.0"
wasmtime-wasi = "24.0"
governor = "0.6"
validator = { version = "0.19", features = ["derive"] }
regex = "1.0"
argon2 = "0.5"
aes-gcm = "0.10"
blake2 = "0.10"
env_logger = "0.10"
either = "1"
num_cpus = "1.0"

# UPDATE THESE:
libp2p = { version = "0.53", features = [
  "tcp", "yamux", "noise", "gossipsub", "identify", 
  "ping", "kad", "request-response", "serde", "mdns", 
  "tokio", "macros",
  "relay",    # Add for NAT traversal
  "dcutr"     # Add for hole punching
] }

ed25519-dalek = { version = "2", default-features = false, features = [
  "std", "rand_core", "batch"  # Add batch verification
] }
```

---

## 🔧 Migration Strategy

1. **Phase 1**: Update workspace `Cargo.toml` with all missing dependencies
2. **Phase 2**: Update each crate to use `{ workspace = true }` (can be done in parallel)
3. **Phase 3**: Run `cargo update` to sync versions
4. **Phase 4**: Test build with `cargo build --workspace`
5. **Phase 5**: Fix any remaining issues

---

**End of Report**
