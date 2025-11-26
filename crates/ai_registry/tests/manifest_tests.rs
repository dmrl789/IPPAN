use ippan_ai_core::deterministic_gbdt::DeterministicGBDT;
use ippan_ai_registry::manifest::{
    canonical_sha256, load_manifest, recompute_inference_hash, validate_inference_hashes,
};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn canonical_manifest_matches_artifacts() {
    let root = repo_root();
    let manifest_path = root.join("models/canonical_manifest.json");
    let manifest = load_manifest(&manifest_path).expect("manifest should parse");
    assert!(!manifest.models.is_empty());

    for entry in &manifest.models {
        // Ensure metadata hash aligns with canonical artifact hash.
        assert_eq!(entry.metadata.id.hash, entry.artifact.sha256);

        // Recompute canonical hash from the artifact bytes.
        let artifact_path = root.join(&entry.artifact.path);
        let model = DeterministicGBDT::from_json_file(&artifact_path)
            .expect("model should load from canonical artifact");
        let canonical_json = model.to_canonical_json().expect("canonical json");
        let computed_hash = canonical_sha256(canonical_json.as_bytes());
        assert_eq!(computed_hash, entry.artifact.sha256);

        // Deterministic inference hashes should be identical for all architectures.
        validate_inference_hashes(entry).expect("architecture hashes should match");

        let recomputed = recompute_inference_hash(entry, &root).expect("recompute hash");
        for (arch, hash) in &entry.inference.architectures {
            assert_eq!(hash, &recomputed, "architecture {arch} mismatch");
        }
    }
}

#[test]
fn architecture_hash_files_match_manifest() {
    let root = repo_root();
    let manifest_path = root.join("models/canonical_manifest.json");
    let manifest = load_manifest(&manifest_path).expect("manifest should parse");
    let entry = &manifest.models[0];

    let architectures = ["x86_64", "aarch64"];
    for arch in architectures {
        let file_path = root.join(format!("models/deterministic_gbdt_model.{arch}.sha256"));
        let contents = fs::read_to_string(&file_path).expect("hash file must exist");
        assert_eq!(contents.trim(), entry.artifact.sha256);
    }
}
