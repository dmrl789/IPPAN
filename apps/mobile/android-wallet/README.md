# IPPAN Android Wallet

A Jetpack Compose reference wallet for the IPPAN network. The module focuses on
showing how the app boots, persists keys with the Android Keystore, talks to
multiple IPPAN HTTP nodes, and renders wallet data with Material 3 components.
It is not yet a feature complete production wallet, but it provides a solid
baseline for teams that want to continue the build out.

## Current capabilities

- ✅ Hardware-backed key generation and storage via `CryptoUtils` and
  `SecureKeyStorage`
- ✅ Multi-endpoint REST client with automatic failover (`IppanApiClient`)
- ✅ Wallet state flow exposed through `WalletRepository` and
  `WalletViewModel`
- ✅ Compose UI for overview, history, settings, and send flows
- ✅ Biometric authentication helper (not yet wired into the send flow)
- ✅ Snapshot tests (Paparazzi) for each screen to assist design reviews

## Roadmap and known gaps

The following items still need implementation before the wallet is production
ready:

- 🔲 Real fiat conversion rates – a placeholder multiplier is used in the data
  layer until price feeds are available.
- 🔲 Push notifications and background sync.
- 🔲 Camera / QR code scanning wiring in the send sheet.
- 🔲 Certificate pinning and custom TLS trust management.
- 🔲 Comprehensive unit/UI test coverage and Play Store hardening.

See [`PRODUCTION_STATUS.md`](PRODUCTION_STATUS.md) for a more detailed checklist.

## Project structure

```
app/
├── src/main/java/org/ippan/wallet/
│   ├── MainActivity.kt                # Compose navigation shell
│   ├── WalletViewModel.kt             # State, intents and validation
│   ├── crypto/                        # Key generation & signing helpers
│   ├── data/                          # Repository and domain models
│   ├── network/                       # OkHttp powered IPPAN client
│   ├── security/                      # Secure storage & biometrics
│   └── ui/components/                 # Screen level Compose components
├── src/test/java/                     # JVM unit tests & Paparazzi snapshots
└── src/androidTest/java/              # Instrumented tests (placeholder)
```

## Getting started

> **Note:** The Gradle wrapper JAR is stored with [Git LFS](https://git-lfs.com/).
> Make sure LFS is installed (`git lfs install`) before cloning or running the
> wrapper so the build tooling is pulled down automatically.

1. Open the `apps/mobile/android-wallet` folder in Android Studio Hedgehog or
   newer.
2. When prompted, let the IDE download the Android Gradle Plugin and all
   dependencies.
3. Configure the list of preferred IPPAN nodes in
   [`app/src/main/res/values/nodes.xml`](app/src/main/res/values/nodes.xml).
4. Build and run the `app` configuration on an API 26+ device or emulator.

## Generating screenshots

The project ships with [Paparazzi](https://github.com/cashapp/paparazzi)
snapshot tests. Once dependencies have been resolved, run:

```bash
./gradlew :app:paparazziDebug
```

The resulting PNG files are stored under
`app/build/reports/paparazzi/images/`. Copy the desired renders into the
`docs/screenshots/` folder (see below) when updating documentation or release
notes.

## Testing

| Type            | Command                            | Notes                                      |
| --------------- | ---------------------------------- | ------------------------------------------ |
| Unit tests      | `./gradlew :app:testDebugUnitTest` | Requires Android Gradle Plugin and SDK     |
| Snapshot tests  | `./gradlew :app:paparazziDebug`    | Produces Compose renders without emulators |
| Instrumentation | `./gradlew :app:connectedCheck`    | Requires an attached device/emulator       |

## Screenshots

Generated screenshots live in [`docs/screenshots/`](docs/screenshots/) so that
non-technical collaborators can review the UI without building the app. Run the
Paparazzi task above and copy the latest renders into that directory before
committing visual changes.


## Automated release builds

Publishing a GitHub Release (or manually running the workflow) will trigger the
[`Android Wallet Release Build`](../../../.github/workflows/android-wallet-release.yml)
workflow. The job provisions the Android SDK, builds the release APK via
`./gradlew :app:assembleRelease`, uploads the artifact for inspection, and—when
triggered by a tagged release—attaches the APK to the corresponding GitHub
Release.
