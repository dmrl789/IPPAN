#!/usr/bin/env bash
set -euo pipefail

# Download the prebuilt verify_model_hash binary for a branch/tag and run it
# against the given DLC config. Avoids installing cargo/rustup on nodes.
#
# Usage:
#   node-verify-model-hash.sh /etc/ippan/config/dlc.toml [ref]
#   env VERIFY_MODEL_HASH_REF=<ref> node-verify-model-hash.sh /etc/ippan/config/dlc.toml
#
# - ref defaults to "master" (latest successful run of ci.yml on master)
# - Requires: curl, tar, and either unzip or python3 (for zip extraction)

CONFIG_PATH="${1:-}"
REF="${VERIFY_MODEL_HASH_REF:-${2:-master}}"

if [[ -z "${CONFIG_PATH}" ]]; then
  echo "Usage: $0 /path/to/dlc.toml [ref]" >&2
  exit 1
fi

if [[ ! -f "${CONFIG_PATH}" ]]; then
  echo "Config not found: ${CONFIG_PATH}" >&2
  exit 1
fi

WORKFLOW="ci.yml"
ARTIFACT_BASENAME="verify-model-hash-${REF}-x86_64-unknown-linux-gnu"
ARTIFACT_URL="https://nightly.link/dmrl789/IPPAN/workflows/${WORKFLOW}/${REF}/${ARTIFACT_BASENAME}.zip"

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "${TMP_DIR}"; }
trap cleanup EXIT

echo "Downloading ${ARTIFACT_URL}"
curl -fL --retry 3 --retry-delay 2 "${ARTIFACT_URL}" -o "${TMP_DIR}/artifact.zip"

if command -v unzip >/dev/null 2>&1; then
  unzip -q "${TMP_DIR}/artifact.zip" -d "${TMP_DIR}"
elif command -v python3 >/dev/null 2>&1; then
  python3 - "${TMP_DIR}/artifact.zip" "${TMP_DIR}" <<'PY'
import sys
import zipfile

zip_path, out_dir = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(zip_path) as z:
    z.extractall(out_dir)
PY
else
  echo "Need unzip or python3 to extract artifact.zip" >&2
  exit 1
fi

tar -C "${TMP_DIR}" -xzf "${TMP_DIR}/${ARTIFACT_BASENAME}.tar.gz"
chmod +x "${TMP_DIR}/verify_model_hash"

echo "Running verify_model_hash against ${CONFIG_PATH} (ref=${REF})"
"${TMP_DIR}/verify_model_hash" "${CONFIG_PATH}"

