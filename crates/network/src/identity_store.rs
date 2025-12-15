use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use libp2p::identity;
use tempfile::NamedTempFile;

const DEFAULT_IDENTITY_PATH: &str = "/var/lib/ippan/p2p/identity.key";
const FALLBACK_PATHS: [&str; 4] = [
    "/var/lib/ippan/p2p/identity.key",
    "/var/lib/ippan/p2p_key",
    "/var/lib/ippan/node_key",
    "/etc/ippan/p2p_key",
];

/// Load an existing libp2p identity keypair from `path` or generate and persist a new one.
pub fn load_or_generate_identity_keypair(path: &Path) -> Result<identity::Keypair> {
    if path.exists() {
        return read_identity_keypair(path);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create identity dir {}", parent.display()))?;
    }

    let keypair = identity::Keypair::generate_ed25519();
    persist_identity_keypair(&keypair, path)?;
    Ok(keypair)
}

/// Resolve an identity keypair using configured and legacy paths.
///
/// Order:
/// 1. Configured path (if provided)
/// 2. /var/lib/ippan/p2p/identity.key
/// 3. /var/lib/ippan/p2p_key
/// 4. /var/lib/ippan/node_key
/// 5. /etc/ippan/p2p_key
///
/// The first existing path wins. If none exist, a new key is created at the configured
/// path when provided, otherwise at the default path (#2).
pub fn load_identity_with_fallback(
    configured: Option<&Path>,
) -> Result<(PathBuf, identity::Keypair)> {
    let default_path = PathBuf::from(DEFAULT_IDENTITY_PATH);
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(path) = configured {
        candidates.push(path.to_path_buf());
    }
    candidates.push(default_path.clone());
    for legacy in FALLBACK_PATHS.iter().skip(1) {
        candidates.push(PathBuf::from(legacy));
    }

    if let Some(existing) = candidates.iter().find(|p| p.is_file()) {
        let keypair = read_identity_keypair(existing)?;
        return Ok((existing.clone(), keypair));
    }

    let target = configured.map(PathBuf::from).unwrap_or(default_path);
    let keypair = load_or_generate_identity_keypair(&target)?;
    Ok((target, keypair))
}

fn read_identity_keypair(path: &Path) -> Result<identity::Keypair> {
    let raw = fs::read(path).with_context(|| format!("read identity key at {}", path.display()))?;
    identity::Keypair::from_protobuf_encoding(&raw)
        .map_err(|e| anyhow!("decode identity key at {} failed: {e}", path.display()))
}

fn persist_identity_keypair(keypair: &identity::Keypair, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    fs::create_dir_all(&parent)
        .with_context(|| format!("create identity parent dir {}", parent.display()))?;

    let encoded = keypair
        .to_protobuf_encoding()
        .map_err(|e| anyhow!("encode libp2p identity keypair: {e}"))?;

    let mut temp = NamedTempFile::new_in(&parent)
        .with_context(|| format!("create temp identity file in {}", parent.display()))?;
    temp.write_all(&encoded)
        .with_context(|| format!("write temp identity file in {}", parent.display()))?;
    temp.as_file()
        .sync_all()
        .with_context(|| format!("sync temp identity file in {}", parent.display()))?;

    temp.persist(path).map_err(|e| {
        anyhow!(
            "persist identity key to {} failed: {}",
            path.display(),
            e.error
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_persists_and_reuses_identity_keypair() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let key_path = temp_dir.path().join("identity.key");

        let (resolved_first, first_key) = load_identity_with_fallback(Some(key_path.as_path()))?;
        assert_eq!(resolved_first, key_path);
        assert!(key_path.exists());

        let first_public = first_key.public();
        let (_, second_key) = load_identity_with_fallback(Some(key_path.as_path()))?;
        assert_eq!(second_key.public(), first_public);

        Ok(())
    }
}
