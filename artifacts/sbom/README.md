## SBOM Artifacts

This directory contains the Software Bill of Materials generated from the current workspace using `cargo sbom`.

- `ippan-sbom.spdx.json` – SPDX 2.3 JSON SBOM for the entire workspace
- `ippan-sbom.spdx.json.sig` – Detached RSA signature (`openssl dgst -sha256 -sign`)
- `ippan-sbom-signing-public.pem` – Public key for verifying the signature

### Verification

```bash
openssl dgst -sha256 \
  -verify artifacts/sbom/ippan-sbom-signing-public.pem \
  -signature artifacts/sbom/ippan-sbom.spdx.json.sig \
  artifacts/sbom/ippan-sbom.spdx.json
```

### Regenerating the SBOM

```bash
cargo sbom --project-directory . --output-format spdx_json_2_3 > artifacts/sbom/ippan-sbom.spdx.json
openssl dgst -sha256 -sign /path/to/sbom_signing_private.pem \
  -out artifacts/sbom/ippan-sbom.spdx.json.sig \
  artifacts/sbom/ippan-sbom.spdx.json
openssl rsa -in /path/to/sbom_signing_private.pem -pubout \
  -out artifacts/sbom/ippan-sbom-signing-public.pem
```

> ⚠️ The private key used for signing is **not** stored in the repository. After signing, move `/tmp/sbom_signing_private.pem` (or your chosen key path) to a secure secret manager.
