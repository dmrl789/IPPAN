#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "${ROOT_DIR}"

python3 <<'PY'
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover - fallback for older runtimes
    try:
        import tomli as tomllib  # type: ignore
    except ModuleNotFoundError as exc:  # pragma: no cover - ensure actionable error
        raise SystemExit(
            "Python 3.11+ or the 'tomli' package is required to parse Cargo.toml"
        ) from exc

workspace_toml = Path("Cargo.toml")
if not workspace_toml.exists():
    sys.stderr.write("Cargo.toml not found at repository root.\n")
    sys.exit(1)

with workspace_toml.open("rb") as fp:
    data = tomllib.load(fp)

workspace_pkg = data.get("workspace", {}).get("package", {})
ws_metadata = data.get("workspace", {}).get("metadata", {})

package_version = workspace_pkg.get("version")
metadata_version = ws_metadata.get("version")

if not package_version:
    sys.stderr.write("[workspace.package] version is missing in Cargo.toml.\n")
    sys.exit(1)

if not metadata_version:
    sys.stderr.write("[workspace.metadata] version is missing in Cargo.toml.\n")
    sys.exit(1)

if package_version != metadata_version:
    sys.stderr.write(
        f"Version mismatch: workspace.package.version={package_version} "
        f"!= workspace.metadata.version={metadata_version}.\n"
    )
    sys.exit(1)

version = package_version
tag_name = f"v{version}"

tag_list = subprocess.run(
    ["git", "tag", "-l", tag_name],
    capture_output=True,
    text=True,
    check=False,
)

tags = [t.strip() for t in tag_list.stdout.splitlines() if t.strip()]
if tag_name in tags:
    sys.stderr.write(f"Tag {tag_name} already exists. Bump version before releasing.\n")
    sys.exit(1)

changelog_path = Path("CHANGELOG.md")
if not changelog_path.exists():
    sys.stderr.write("CHANGELOG.md is missing.\n")
    sys.exit(1)

changelog_text = changelog_path.read_text(encoding="utf-8")
pattern = re.compile(rf"^##\s+\[?v?{re.escape(version)}\]?", re.MULTILINE)

if not pattern.search(changelog_text):
    sys.stderr.write(
        f"CHANGELOG.md does not contain a section for version {version}.\n"
    )
    sys.exit(1)

print(f"Version governance checks passed for {version} (no existing tag, changelog entry present).")
PY

echo "Running reproducible build: cargo build --workspace --locked --release"
cargo build --workspace --locked --release

if command -v cargo-deny >/dev/null 2>&1; then
  echo "Running license audit: cargo deny check licenses"
  cargo deny check licenses
else
  echo "cargo-deny is not installed. Install it via 'cargo install --locked cargo-deny'." >&2
  exit 1
fi

echo "Release checklist completed successfully."
