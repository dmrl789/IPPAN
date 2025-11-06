use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));

    let workspace_dir = env::var("CARGO_WORKSPACE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| find_workspace_root(&manifest_dir));

    let cargo_toml = workspace_dir.join("Cargo.toml");

    println!("cargo:rerun-if-changed={}", cargo_toml.display());

    let cargo_toml_contents = fs::read_to_string(&cargo_toml)
        .unwrap_or_else(|err| panic!("Failed to read {}: {}", cargo_toml.display(), err));

    let workspace: toml::Value = toml::from_str(&cargo_toml_contents)
        .unwrap_or_else(|err| panic!("Failed to parse {}: {}", cargo_toml.display(), err));

    let workspace_table = workspace
        .get("workspace")
        .unwrap_or_else(|| panic!("Missing [workspace] table in {}", cargo_toml.display()));

    let metadata_table = workspace_table.get("metadata").unwrap_or_else(|| {
        panic!(
            "Missing [workspace.metadata] table in {}",
            cargo_toml.display()
        )
    });

    let ippan_version = metadata_table
        .get("ippan_version")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| panic!("Missing `ippan_version` in [workspace.metadata]"));

    if let Some(package_table) = workspace_table.get("package") {
        if let Some(package_version) = package_table
            .get("version")
            .and_then(|value| value.as_str())
        {
            assert_eq!(
                package_version, ippan_version,
                "[workspace.package].version ({}) does not match [workspace.metadata].ippan_version ({})",
                package_version, ippan_version
            );
        }
    }

    println!("cargo:rustc-env=IPPAN_VERSION={}", ippan_version);
}

fn find_workspace_root(start: &Path) -> PathBuf {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(contents) = fs::read_to_string(&candidate) {
                if let Ok(value) = toml::from_str::<toml::Value>(&contents) {
                    if value.get("workspace").is_some() {
                        return ancestor.to_path_buf();
                    }
                }
            }
        }
    }

    panic!(
        "Unable to locate workspace root containing Cargo.toml starting from {}",
        start.display()
    );
}
