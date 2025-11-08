//! Canonical AI model manifest generation utilities.
//!
//! Provides helper structures and functions for producing reproducible
//! manifests describing deterministic AI model artifacts. The manifest is
//! consumed by governance tooling to ensure that every node verifies the
//! exact same bytes regardless of CPU architecture.

use ippan_ai_core::deterministic_gbdt::{DeterministicGBDT, DeterministicGBDTError};
use ippan_ai_core::serialization::canonical_json_string;
use ippan_ai_core::types::ModelMetadata;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Canonical hash seed used when deriving deterministic inference hashes.
pub const DEFAULT_INFERENCE_SEED: &str = "ippan::deterministic_inference::v1";

/// Description of a single model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelArtifact {
    /// Path to the artifact relative to the repository root.
    pub path: String,
    /// Artifact format identifier (e.g. `deterministic_gbdt.canonical_json`).
    pub format: String,
    /// SHA-256 hash of the canonical artifact bytes.
    pub sha256: String,
    /// Blake3 hash of the canonical artifact bytes.
    pub blake3: String,
    /// Size of the canonical artifact bytes.
    pub size_bytes: u64,
}

/// Deterministic inference hash values for supported CPU architectures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeterministicInferenceHashes {
    /// Hash seed used when computing the inference certificate.
    pub seed: String,
    /// Mapping from architecture identifier to the resulting hash.
    pub architectures: BTreeMap<String, String>,
}

/// Manifest entry describing a single deterministic model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifestEntry {
    /// Model metadata used by the on-chain registry.
    pub metadata: ModelMetadata,
    /// Canonical artifact information.
    pub artifact: ModelArtifact,
    /// Deterministic inference hash certificates per architecture.
    pub inference: DeterministicInferenceHashes,
}

/// Canonical manifest describing all deterministic models bundled with the
/// node release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Manifest schema version.
    pub manifest_version: u32,
    /// ISO-8601 timestamp describing when the manifest was generated.
    pub generated_at: String,
    /// All bundled models.
    pub models: Vec<ModelManifestEntry>,
}

impl ModelManifest {
    /// Create a new manifest instance.
    pub fn new(
        manifest_version: u32,
        generated_at: String,
        models: Vec<ModelManifestEntry>,
    ) -> Self {
        Self {
            manifest_version,
            generated_at,
            models,
        }
    }

    /// Serialize the manifest to canonical JSON and write it to disk.
    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let json = canonical_json_string(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

impl ModelManifestEntry {
    /// Build a manifest entry from a deterministic GBDT model stored on disk.
    pub fn from_deterministic_gbdt<P: AsRef<Path>>(
        mut metadata: ModelMetadata,
        model_path: P,
        repository_root: P,
        inference_seed: &str,
    ) -> anyhow::Result<Self> {
        let model_path = model_path.as_ref();
        let repo_root = repository_root.as_ref();

        let full_path = if model_path.is_absolute() {
            model_path.to_path_buf()
        } else {
            repo_root.join(model_path)
        };

        let model = DeterministicGBDT::from_json_file(&full_path)
            .map_err(|err| anyhow::anyhow!("failed to load model: {err}"))?;
        let canonical_json = model
            .to_canonical_json()
            .map_err(|err| anyhow::anyhow!("failed to canonicalize model: {err}"))?;

        let sha256 = canonical_sha256(canonical_json.as_bytes());
        let blake3 = canonical_blake3(canonical_json.as_bytes());
        let size_bytes = canonical_json.len() as u64;

        // Update metadata with canonical measurements.
        metadata.id.hash = sha256.clone();
        metadata.size_bytes = size_bytes;
        metadata.parameter_count = model.trees.iter().map(|tree| tree.nodes.len() as u64).sum();
        metadata.input_shape = vec![infer_feature_count(&model) as usize];
        metadata.output_shape = vec![1];

        let mut architectures = BTreeMap::new();
        let inference_hash = model
            .model_hash(inference_seed)
            .map_err(|err| anyhow::anyhow!("failed to compute deterministic hash: {err}"))?;
        architectures.insert("x86_64".to_string(), inference_hash.clone());
        architectures.insert("aarch64".to_string(), inference_hash);

        let relative_path = normalize_relative_path(model_path, repo_root)?;
        let artifact = ModelArtifact {
            path: relative_path,
            format: "deterministic_gbdt.canonical_json".to_string(),
            sha256,
            blake3,
            size_bytes,
        };

        let inference = DeterministicInferenceHashes {
            seed: inference_seed.to_string(),
            architectures,
        };

        Ok(Self {
            metadata,
            artifact,
            inference,
        })
    }

    /// Convenience accessor for the canonical SHA-256 hash.
    pub fn canonical_sha256(&self) -> &str {
        &self.artifact.sha256
    }
}

/// Compute the SHA-256 hash for the provided bytes.
pub fn canonical_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute the Blake3 hash for the provided bytes.
pub fn canonical_blake3(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

fn normalize_relative_path(path: &Path, repo_root: &Path) -> anyhow::Result<String> {
    let canonical_root = repo_root
        .canonicalize()
        .map_err(|err| anyhow::anyhow!("failed to canonicalize repo root: {err}"))?;

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_root.join(path)
    };

    let relative = pathdiff::diff_paths(&absolute_path, &canonical_root)
        .ok_or_else(|| anyhow::anyhow!("failed to compute relative path"))?;

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn infer_feature_count(model: &DeterministicGBDT) -> u32 {
    model
        .trees
        .iter()
        .flat_map(|tree| tree.nodes.iter().map(|node| node.feature as u32))
        .max()
        .map(|max_feature| max_feature + 1)
        .unwrap_or(0)
}

/// Update helper that rewrites architecture-specific hash files with the
/// canonical SHA-256 digest.
pub fn write_architecture_hash_files(
    entry: &ModelManifestEntry,
    repo_root: impl AsRef<Path>,
    architectures: &[(String, PathBuf)],
) -> anyhow::Result<()> {
    let repo_root = repo_root.as_ref();
    for (arch, relative_path) in architectures {
        let path = if relative_path.is_absolute() {
            relative_path.clone()
        } else {
            repo_root.join(relative_path)
        };
        fs::write(&path, format!("{}\n", entry.canonical_sha256()))?;
        tracing::info!(arch = arch.as_str(), path = %path.display(), "updated canonical hash");
    }
    Ok(())
}

/// Convenience function to load an existing manifest from disk.
pub fn load_manifest<P: AsRef<Path>>(path: P) -> anyhow::Result<ModelManifest> {
    let data = fs::read_to_string(path)?;
    let manifest: ModelManifest = serde_json::from_str(&data)?;
    Ok(manifest)
}

/// Validate that all architectures share the same deterministic inference hash.
pub fn validate_inference_hashes(entry: &ModelManifestEntry) -> anyhow::Result<()> {
    let mut iter = entry.inference.architectures.values();
    if let Some(first) = iter.next() {
        if iter.any(|hash| hash != first) {
            anyhow::bail!(
                "architecture hashes diverge for model {}",
                entry.metadata.id.name
            );
        }
    }
    Ok(())
}

/// Helper to recompute the deterministic inference hash for a manifest entry
/// using the canonical artifact bytes.
pub fn recompute_inference_hash(
    entry: &ModelManifestEntry,
    repo_root: impl AsRef<Path>,
) -> Result<String, DeterministicGBDTError> {
    let repo_root = repo_root.as_ref();
    let path = repo_root.join(&entry.artifact.path);
    let model = DeterministicGBDT::from_json_file(&path)?;
    model.model_hash(&entry.inference.seed)
}
