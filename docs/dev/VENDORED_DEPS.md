# Vendored dependencies (developer notes)

## Python wheels (supported)

Python training dependencies are vendored under:

- ai_assets/vendor/pip_wheels/ (Git LFS)

## Rust crates (optional)

We do NOT force Cargo to use a vendored endor/cargo directory by default.

Reason: Windows filesystems cannot represent certain filenames used by crates (e.g. AUX.*),
which prevents committing a complete vendor directory from Windows.

### If you need fully-offline Rust builds

Do this from WSL on an ext4 filesystem (not /mnt/c):

1) Run: cargo vendor vendor/cargo > .cargo/config.vendor.example.toml

2) Copy: .cargo/config.vendor.example.toml -> .cargo/config.toml (LOCAL ONLY)

3) Do not commit .cargo/config.toml
