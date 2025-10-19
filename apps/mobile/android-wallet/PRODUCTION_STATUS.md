# IPPAN Android Wallet – Production Readiness Snapshot

_Last updated: October 2025_

The Android wallet is **pre-production**. The foundations (secure key storage,
network client, Compose UI) are in place, but several operational and compliance
tasks remain before a public release. Use the checklist below to track progress.

## Build & Tooling

| Item | Status | Notes |
| ---- | ------ | ----- |
| Gradle wrapper & settings | ⚠️ In progress | Wrapper JAR must be generated locally; AGP download required |
| CI pipelines | ⛔ Not started | No workflows for lint/tests/builds |
| Release configuration | ⚠️ In progress | ProGuard file present, signing config not defined |

## Security

| Item | Status | Notes |
| ---- | ------ | ----- |
| Hardware backed keys | ✅ Complete | Generated with Android Keystore (secp256r1) |
| Biometric gating | ⚠️ In progress | Helper exists, not wired into send flow |
| Certificate pinning | ⛔ Not started | HTTPS relies on system trust store |
| Encrypted preferences | ✅ Complete | `EncryptedSharedPreferences` wraps wallet metadata |

## Blockchain integration

| Item | Status | Notes |
| ---- | ------ | ----- |
| Multi-node failover | ✅ Complete | `IppanApiClient` rotates through configured nodes |
| Balance & history | ✅ Complete | REST endpoints mapped to domain models |
| Transaction submit | ✅ Complete | Hashing, signing, and broadcast implemented |
| Gas price discovery | ✅ Complete | Uses `/api/gas-price` response |
| Fiat conversion | ⚠️ Placeholder | Static multiplier until real oracle is available |

## User experience

| Item | Status | Notes |
| ---- | ------ | ----- |
| Compose navigation shell | ✅ Complete | Overview, Activity, Settings, Send sheet |
| QR / camera integration | ⛔ Not started | UI component present but not hooked |
| Error & empty states | ⚠️ In progress | Toast/snackbar feedback only |
| Accessibility review | ⛔ Not started | No TalkBack or font scaling audits |
| Localization | ⛔ Not started | English-only strings |

## Quality

| Item | Status | Notes |
| ---- | ------ | ----- |
| JVM unit tests | ⚠️ In progress | API client tests and snapshots available |
| Instrumented tests | ⛔ Not started | Need Compose UI tests on device |
| Snapshot coverage | ✅ Complete | Paparazzi renders for each screen |
| Performance profiling | ⛔ Not started | No baseline profiles |

## Next steps

1. Bring the Gradle wrapper JAR under version control and wire up CI so the
   project can be built outside Android Studio.
2. Hook the biometric prompt into the transfer submission flow and add negative
   path instrumentation tests.
3. Replace the placeholder fiat conversion with a price service integration.
4. Implement QR code scanning and add feature flag driven by `SecureKeyStorage`.
5. Add crash reporting, logging, and analytics guards for production rollout.

