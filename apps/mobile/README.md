# IPPAN Mobile Wallet

This directory contains the Android wallet prototype for IPPAN. The project
uses Jetpack Compose and Material 3 to deliver a modern, reactive client that
can run in the Android emulator without depending on a live blockchain node.

## Project layout

```
apps/mobile/android-wallet
├── build.gradle.kts           # Gradle configuration (plugins + versions)
├── settings.gradle.kts        # Includes the :app module
├── gradle.properties          # AndroidX + Kotlin defaults
└── app
    ├── build.gradle.kts       # Application module configuration
    ├── proguard-rules.pro
    └── src/main
        ├── AndroidManifest.xml
        ├── java/org/ippan/wallet
        │   ├── MainActivity.kt
        │   ├── WalletViewModel.kt
        │   ├── data/…         # Fake repository + models
        │   └── ui/components  # Composable screens + sheets
        └── res
            ├── drawable/
            └── values/
```

## Getting started

1. Ensure you have Android Studio Hedgehog (or newer) with the Compose tooling.
2. From the IDE welcome screen choose **Open** and select
   `apps/mobile/android-wallet`.
3. Let Gradle sync; the project targets **compileSdk 34** and **minSdk 26**.
4. Use the **Run** button to launch the `app` configuration on an emulator or
   physical device.

The app boots into the overview tab, showing a mocked wallet snapshot. The fake
repository (`FakeWalletRepository`) produces balances and transactions so the
UI can be tested offline. Use the floating **Send** action to open the send
sheet and simulate a transaction; the snackbar will confirm the submission and
update the in-memory balances.

## Next steps

* Replace `FakeWalletRepository` with a real implementation that talks to the
  gateway REST endpoints.
* Wire the refresh action to the chain so that it re-syncs balances and
  transaction history.
* Harden the form validation, adding QR code scanning and address book support.
* Integrate biometric prompts and encrypted key management once backend
  custody endpoints are available.
