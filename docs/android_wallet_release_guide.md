# IPPAN Android Wallet Project & Release Workflow

This guide describes a reference Android implementation for the IPPAN wallet and the CI/CD pipeline that automatically builds and publishes release binaries to GitHub Releases. It is intended for teams that want a reproducible, auditable path from source to signed APKs without committing build artifacts to the repository.

---

## 1. Repository Layout

```
apps/
└── android-wallet/
    ├── app/
    │   ├── build.gradle.kts
    │   ├── src/
    │   │   ├── main/
    │   │   │   ├── AndroidManifest.xml
    │   │   │   ├── java/
    │   │   │   │   └── net/ippan/wallet/
    │   │   │   │       ├── MainActivity.kt
    │   │   │   │       ├── navigation/
    │   │   │   │       ├── ui/components/
    │   │   │   │       └── viewmodel/
    │   │   │   └── res/
    │   │   │       ├── layout/
    │   │   │       ├── values/
    │   │   │       └── drawable/
    │   │   └── androidTest/
    │   │       └── ...
    │   └── proguard-rules.pro
    ├── build.gradle.kts
    ├── gradle.properties
    ├── settings.gradle.kts
    ├── rust/
    │   ├── Cargo.toml
    │   ├── build.rs
    │   └── src/lib.rs
    └── scripts/
        └── prepare-rust.sh
```

### Key Notes

* The Android UI uses Jetpack Compose inside `MainActivity.kt` and the `ui/components` package.
* Rust signing logic lives in `apps/android-wallet/rust` and is compiled via Cargo with the `cargo-ndk` toolchain.
* JNI bindings are generated through UniFFI and land in `rust/src/lib.rs`, which exposes `WalletClient` and `HashTimer` helpers to Kotlin.
* Use Gradle version catalogs in `gradle/libs.versions.toml` to lock library versions and keep builds deterministic.

---

## 2. Build Scripts

### 2.1 Rust Preparation

`apps/android-wallet/scripts/prepare-rust.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."

rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

cargo install cargo-ndk --locked

pushd "${PROJECT_ROOT}/rust"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -- build --release
popd
```

The script pins the Android targets, installs `cargo-ndk`, and emits JNI libraries into `apps/android-wallet/app/src/main/jniLibs/`.

### 2.2 Gradle Build

Run the build from the repository root:

```bash
./gradlew :apps:android-wallet:app:assembleRelease
```

Gradle should produce `apps/android-wallet/app/build/outputs/apk/release/android-wallet-release.apk`. Make sure signing configs are wired through environment variables (`IPPAN_KEYSTORE_PATH`, `IPPAN_KEYSTORE_ALIAS`, `IPPAN_KEYSTORE_PASSWORD`, `IPPAN_KEY_PASSWORD`).

---

## 3. GitHub Actions Workflow

Create `.github/workflows/android-wallet.yml`:

```yaml
name: Build Android Wallet

on:
  push:
    tags:
      - 'wallet-v*'
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-22.04

    env:
      JAVA_VERSION: '17'
      RUST_TOOLCHAIN: '1.75.0'
      ORG_GRADLE_PROJECT_signingKeyStore: ${{ secrets.IPPAN_KEYSTORE_PATH }}
      ORG_GRADLE_PROJECT_signingKeyAlias: ${{ secrets.IPPAN_KEYSTORE_ALIAS }}
      ORG_GRADLE_PROJECT_signingKeyStorePassword: ${{ secrets.IPPAN_KEYSTORE_PASSWORD }}
      ORG_GRADLE_PROJECT_signingKeyPassword: ${{ secrets.IPPAN_KEY_PASSWORD }}

    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Set up Java
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: ${{ env.JAVA_VERSION }}

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          components: rustfmt, clippy

      - name: Install Android SDK
        uses: android-actions/setup-android@v3

      - name: Prepare Rust artifacts
        working-directory: apps/android-wallet
        run: scripts/prepare-rust.sh

      - name: Gradle build
        run: ./gradlew :apps:android-wallet:app:assembleRelease --stacktrace

      - name: Upload release artifact
        uses: softprops/action-gh-release@v2
        with:
          files: apps/android-wallet/app/build/outputs/apk/release/android-wallet-release.apk
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

* `wallet-v*` tags (e.g., `wallet-v0.1.0`) trigger reproducible builds.
* `softprops/action-gh-release` attaches the APK to the tag’s GitHub Release automatically.
* Secrets carry the signing keystore path and passwords; the keystore itself should be stored in GitHub Actions secrets or an encrypted artifact, not in the repository.

---

## 4. Local Development Checklist

1. `rustup target add ...` for Android ABIs and verify `cargo ndk --version`.
2. Run `./gradlew :apps:android-wallet:lintDebug` to ensure Kotlin lint passes.
3. Execute `./gradlew :apps:android-wallet:testDebugUnitTest` for JVM unit tests and `connectedDebugAndroidTest` for instrumentation tests.
4. Keep dependencies updated via `./gradlew :apps:android-wallet:dependencies --write-locks` to refresh lockfiles.
5. Confirm the wallet can connect to the IPPAN gateway sandbox at `https://api.ippan.net` and that WebSocket streaming works with the configured API key.

---

## 5. Release Checklist

| Step | Description |
| --- | --- |
| 1 | Merge feature branches into `main` after CI green (Rust + Android tests). |
| 2 | Run `cargo fmt`, `cargo clippy`, and Android lint locally to catch regressions early. |
| 3 | Update `CHANGELOG.md` with wallet-specific entries if necessary. |
| 4 | Tag the release with `wallet-vX.Y.Z` and push the tag. |
| 5 | Monitor the GitHub Action run; verify the APK is attached to the release and signed. |
| 6 | Smoke-test the APK on a real device or emulator before public announcement. |

---

## 6. Security & Compliance

* Store sensitive material (keystore, API credentials) in GitHub Actions secrets or a dedicated secret manager; never commit them.
* Enable Play Integrity / SafetyNet checks inside the app if you plan Play Store distribution.
* Ensure the Rust wallet bindings enforce deterministic HashTimer ordering and validate all DAG responses before rendering to the UI.
* Attach a Software Bill of Materials (SBOM) to releases using `cargo auditable` for Rust and `gradle sbom` plugins for Android dependencies.

---

## 7. Future Enhancements

* Add a second workflow job that builds an Android App Bundle (`.aab`) for Play Store submission.
* Integrate `fastlane supply` to push beta builds to internal testers automatically.
* Expand instrumentation coverage with Espresso tests for send/receive flows.
* Publish checksum files (`SHA256SUMS.txt`) alongside APKs to simplify integrity verification for users.

---

By keeping binaries out of the main branch and relying on automated release workflows, the IPPAN team can distribute signed Android wallets rapidly while maintaining a clean Git history and verifiable build process.
