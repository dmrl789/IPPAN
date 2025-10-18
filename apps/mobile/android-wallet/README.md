# IPPAN Android Wallet - Production Ready

A modern, secure Android wallet for the IPPAN blockchain network built with Jetpack Compose and Material 3.

## 🚀 Features

### Core Functionality
- **🔐 Secure Wallet Management**: ECDSA key generation and secure storage
- **💰 Balance Tracking**: Real-time balance updates from IPPAN network
- **📱 Transaction History**: Complete transaction history with status tracking
- **💸 Send/Receive**: Easy token transfers with QR code scanning
- **🔒 Biometric Security**: Fingerprint/Face ID authentication

### Security Features
- **🛡️ Hardware Security**: Android Keystore integration
- **🔐 Encrypted Storage**: AES-GCM encryption for sensitive data
- **👆 Biometric Authentication**: Secure transaction signing
- **🔑 Key Management**: Secure private key storage and backup

### User Experience
- **🎨 Modern UI**: Material 3 design with dark/light themes
- **📱 Responsive Design**: Optimized for phones and tablets
- **🔄 Real-time Updates**: Live balance and transaction updates
- **📷 QR Scanner**: Easy address sharing and receiving
- **🌐 Network Status**: Connection health monitoring

## 🏗️ Architecture

```
📱 IPPAN Android Wallet
├── 🎨 UI Layer (Jetpack Compose)
│   ├── MainActivity (Navigation)
│   ├── OverviewScreen (Balance/Assets)
│   ├── SendTokenSheet (Transaction Form)
│   ├── QRCodeScanner (Address Scanning)
│   └── ActivityScreen (Transaction History)
├── 🧠 ViewModel Layer
│   └── WalletViewModel (State Management)
├── 📊 Data Layer
│   ├── ProductionWalletRepository (Real Blockchain)
│   ├── IppanApiClient (Network Operations)
│   └── SecureKeyStorage (Encrypted Storage)
├── 🔐 Security Layer
│   ├── CryptoUtils (Cryptographic Operations)
│   ├── BiometricAuthManager (Authentication)
│   └── SecureKeyStorage (Key Management)
└── 🌐 Network Layer
    └── IppanApiClient (Blockchain API)
```

## 🛠️ Setup Instructions

### Prerequisites
- Android Studio Hedgehog (2023.1.1) or newer
- Android SDK 26+ (Android 8.0+)
- Java 17 or newer
- Kotlin 1.9.0+

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/dmrl789/IPPAN.git
   cd IPPAN/apps/mobile/android-wallet
   ```

2. **Open in Android Studio**
   - Launch Android Studio
   - Select "Open an existing project"
   - Navigate to `apps/mobile/android-wallet`
   - Click "Open"

3. **Sync dependencies**
   - Android Studio will automatically sync Gradle
   - Wait for all dependencies to download

4. **Configure IPPAN Network**
   - Update `IppanApiClient` base URL in `network/IppanApiClient.kt`
   - Set your IPPAN node endpoint (e.g., `https://api.ippan.org`)

5. **Build and Run**
   - Connect Android device or start emulator
   - Click "Run" button or press `Shift+F10`

### Configuration

#### Network Configuration
```kotlin
// In IppanApiClient.kt
private val baseUrl = "https://your-ippan-node.com" // Update this
```

#### Security Configuration
```kotlin
// In SecureKeyStorage.kt
private const val KEYSTORE_ALIAS = "ippan_wallet_key" // Customize if needed
```

## 🔧 Development

### Project Structure
```
app/src/main/java/org/ippan/wallet/
├── MainActivity.kt                 # Main app entry point
├── WalletViewModel.kt             # State management
├── data/                          # Data layer
│   ├── Models.kt                  # Data models
│   ├── WalletRepository.kt       # Repository interface
│   ├── FakeWalletRepository.kt   # Mock implementation
│   └── ProductionWalletRepository.kt # Real implementation
├── network/                       # Network layer
│   └── IppanApiClient.kt         # API client
├── crypto/                        # Cryptographic utilities
│   └── CryptoUtils.kt            # Crypto operations
├── security/                      # Security features
│   ├── BiometricAuthManager.kt   # Biometric auth
│   └── SecureKeyStorage.kt      # Secure storage
└── ui/                           # UI components
    ├── components/               # Reusable components
    └── theme/                   # Design system
```

### Key Components

#### WalletViewModel
- Manages wallet state and user interactions
- Handles transaction submission and validation
- Coordinates between UI and data layers

#### ProductionWalletRepository
- Real blockchain integration
- Transaction signing and submission
- Balance and history fetching

#### CryptoUtils
- ECDSA key generation and management
- Transaction signing and verification
- Address generation and validation

#### SecureKeyStorage
- Encrypted private key storage
- Biometric authentication integration
- Secure backup and recovery

## 🧪 Testing

### Unit Tests
```bash
./gradlew test
```

### Integration Tests
```bash
./gradlew connectedAndroidTest
```

### Test Coverage
```bash
./gradlew jacocoTestReport
```

## 🔒 Security Considerations

### Private Key Security
- Private keys are stored in Android Keystore
- Encrypted with hardware-backed security
- Never stored in plain text

### Transaction Signing
- All transactions require biometric authentication
- Private keys never leave the secure hardware
- Signatures generated in secure environment

### Network Security
- All API calls use HTTPS
- Certificate pinning for production
- Request/response validation

## 📱 Production Deployment

### Build Configuration
```kotlin
// In build.gradle.kts
buildTypes {
    getByName("release") {
        isMinifyEnabled = true
        isShrinkResources = true
        proguardFiles(
            getDefaultProguardFile("proguard-android-optimize.txt"),
            "proguard-rules.pro"
        )
    }
}
```

### Signing Configuration
1. Generate signing key:
   ```bash
   keytool -genkey -v -keystore ippan-wallet.keystore -alias ippan-wallet -keyalg RSA -keysize 2048 -validity 10000
   ```

2. Configure signing in `build.gradle.kts`:
   ```kotlin
   android {
       signingConfigs {
           create("release") {
               storeFile = file("ippan-wallet.keystore")
               storePassword = "your_store_password"
               keyAlias = "ippan-wallet"
               keyPassword = "your_key_password"
           }
       }
   }
   ```

### Play Store Deployment
1. Build release APK:
   ```bash
   ./gradlew assembleRelease
   ```

2. Upload to Google Play Console
3. Configure app signing
4. Submit for review

## 🌐 Network Integration

### IPPAN Node Requirements
- REST API endpoints for balance/transactions
- WebSocket support for real-time updates
- CORS configuration for mobile access

### API Endpoints
```
GET /api/balance/{address}          # Get account balance
GET /api/transactions/{address}     # Get transaction history
POST /api/transactions              # Submit transaction
GET /api/status                     # Network status
GET /api/gas-price                  # Current gas price
```

## 🔧 Troubleshooting

### Common Issues

#### Build Errors
- Ensure Android Studio is up to date
- Clean and rebuild project
- Check Java/Kotlin versions

#### Network Issues
- Verify IPPAN node is accessible
- Check network permissions
- Validate API endpoints

#### Security Issues
- Ensure device supports biometrics
- Check Android Keystore availability
- Verify permissions in manifest

### Debug Mode
```kotlin
// Enable debug logging
BuildConfig.DEBUG = true
```

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📞 Support

- **Documentation**: [IPPAN Docs](https://docs.ippan.org)
- **Issues**: [GitHub Issues](https://github.com/dmrl789/IPPAN/issues)
- **Discord**: [IPPAN Community](https://discord.gg/ippan)

---

**⚠️ Security Notice**: This is production-ready software. Always verify the integrity of your builds and use official distribution channels.
