# Model Artifacts

The `models/` directory stores canonical deterministic AI artifacts that are
consumed by `ai_registry` and consensus crates.

## Layout

```
models/
├── dlc/
│   └── dlc_model_example.json
├── canonical_manifest.json
└── ... legacy hashes ...
```

* **Canonical JSON:** Every model is serialized using `ippan-ai-core`'s
  `canonical_model_json` helper. The resulting files contain sorted keys,
  deterministic indentation, and integer-only values.
* **BLAKE3 hash:** The canonical bytes are hashed with BLAKE3. Store the hash in
  `config/dlc.toml` (`expected_hash`) and in commit messages/PR descriptions.
* **Naming convention:** `models/dlc/dlc_model_<descriptor>.json` where the
  descriptor reflects the dataset or training date (e.g. `v20250117`).

## Example

```
[dgbdt]
  [dgbdt.model]
  path = "models/dlc/dlc_model_example.json"
  expected_hash = "c549f359dc77fab3739e571d1d404143ac6c31f652588e8846e3770df8d63c26"
```

This configuration ensures `ai_registry` refuses to load any model whose hash
fails to match the committed artifact.
