# IPPAN Android Wallet - Build Guide

## 🚀 Creating APK for IPPAN Android Wallet

### Prerequisites

Before building the APK, you need to install the following:

#### 1. **Java Development Kit (JDK) 17**
```bash
# Download and install JDK 17 from:
# https://adoptium.net/temurin/releases/?version=17
# Or use package manager:
# Windows: choco install openjdk17
# macOS: brew install openjdk@17
# Ubuntu: sudo apt install openjdk-17-jdk
```

#### 2. **Android Studio** (Recommended)
- Download from: https://developer.android.com/studio
- Includes Android SDK, build tools, and emulator

#### 3. **Android SDK** (If not using Android Studio)
```bash
# Set environment variables:
export ANDROID_HOME=/path/to/android-sdk
export PATH=$PATH:$ANDROID_HOME/tools:$ANDROID_HOME/platform-tools
```

### Build Commands

#### **Debug APK** (For testing)
```bash
# Navigate to project directory
cd apps/mobile/android-wallet

# Build debug APK
./gradlew assembleDebug

# APK will be created at:
# app/build/outputs/apk/debug/app-debug.apk
```

#### **Release APK** (For production)
```bash
# Build release APK (requires signing configuration)
./gradlew assembleRelease

# APK will be created at:
# app/build/outputs/apk/release/app-release.apk
```

#### **Android App Bundle (AAB)** (For Play Store)
```bash
# Build release bundle
./gradlew bundleRelease

# AAB will be created at:
# app/build/outputs/bundle/release/app-release.aab
```

### Build Configuration

#### **Debug Build**
- **Minification**: Disabled
- **Signing**: Debug keystore (automatic)
- **Optimization**: None
- **Size**: Larger file size
- **Use case**: Development and testing

#### **Release Build**
- **Minification**: Enabled (ProGuard)
- **Signing**: Release keystore (manual setup required)
- **Optimization**: Full optimization
- **Size**: Smaller file size
- **Use case**: Production deployment

### Signing Configuration

#### **Debug Signing** (Automatic)
```kotlin
// Already configured in build.gradle.kts
debug {
    applicationIdSuffix = ".debug"
    versionNameSuffix = "-dev"
}
```

#### **Release Signing** (Manual Setup Required)
```kotlin
// Create keystore first:
keytool -genkey -v -keystore release.keystore -alias ippan-release -keyalg RSA -keysize 2048 -validity 10000

// Configure in app/build.gradle.kts:
signingConfigs {
    create("release") {
        keyAlias = "ippan-release"
        keyPassword = "your-key-password"
        storeFile = file("release.keystore")
        storePassword = "your-store-password"
    }
}
```

### Environment Setup

#### **Windows**
```powershell
# Set JAVA_HOME
$env:JAVA_HOME = "C:\Program Files\Java\jdk-17"

# Set ANDROID_HOME
$env:ANDROID_HOME = "C:\Users\$env:USERNAME\AppData\Local\Android\Sdk"

# Add to PATH
$env:PATH += ";$env:JAVA_HOME\bin;$env:ANDROID_HOME\tools;$env:ANDROID_HOME\platform-tools"
```

#### **macOS/Linux**
```bash
# Add to ~/.bashrc or ~/.zshrc
export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME=/path/to/android-sdk
export PATH=$PATH:$JAVA_HOME/bin:$ANDROID_HOME/tools:$ANDROID_HOME/platform-tools
```

### Build Process

#### **Step 1: Clean Build**
```bash
./gradlew clean
```

#### **Step 2: Run Tests**
```bash
./gradlew test
./gradlew lint
```

#### **Step 3: Build APK**
```bash
# Debug APK
./gradlew assembleDebug

# Release APK (requires signing)
./gradlew assembleRelease
```

#### **Step 4: Verify Build**
```bash
# Check APK size and contents
ls -la app/build/outputs/apk/debug/
ls -la app/build/outputs/apk/release/

# Install on device (if connected)
adb install app/build/outputs/apk/debug/app-debug.apk
```

### Troubleshooting

#### **Common Issues**

1. **"JAVA_HOME is not set"**
   ```bash
   # Solution: Set JAVA_HOME environment variable
   export JAVA_HOME=/path/to/jdk-17
   ```

2. **"Android SDK not found"**
   ```bash
   # Solution: Set ANDROID_HOME and install SDK
   export ANDROID_HOME=/path/to/android-sdk
   sdkmanager "platform-tools" "platforms;android-34"
   ```

3. **"Gradle daemon not running"**
   ```bash
   # Solution: Start Gradle daemon
   ./gradlew --daemon
   ```

4. **"Build failed with ProGuard"**
   ```bash
   # Solution: Check ProGuard rules in proguard-rules.pro
   # Add keep rules for classes that should not be obfuscated
   ```

#### **Build Optimization**

1. **Enable Gradle Build Cache**
   ```bash
   # Add to gradle.properties
   org.gradle.caching=true
   org.gradle.parallel=true
   ```

2. **Use Gradle Wrapper**
   ```bash
   # Always use ./gradlew instead of gradle
   ./gradlew assembleDebug
   ```

3. **Clean Build When Needed**
   ```bash
   # If build issues persist
   ./gradlew clean build
   ```

### Output Files

#### **Debug APK**
- **Location**: `app/build/outputs/apk/debug/app-debug.apk`
- **Size**: ~15-25 MB
- **Signing**: Debug keystore
- **Installation**: `adb install app-debug.apk`

#### **Release APK**
- **Location**: `app/build/outputs/apk/release/app-release.apk`
- **Size**: ~8-15 MB (optimized)
- **Signing**: Release keystore
- **Installation**: Manual or Play Store

#### **Android App Bundle (AAB)**
- **Location**: `app/build/outputs/bundle/release/app-release.aab`
- **Size**: ~5-10 MB (Play Store optimized)
- **Upload**: Google Play Console

### Testing the APK

#### **Install on Device**
```bash
# Enable USB debugging on Android device
# Connect device via USB
adb devices
adb install app/build/outputs/apk/debug/app-debug.apk
```

#### **Install on Emulator**
```bash
# Start Android emulator
emulator -avd Pixel_7_API_34

# Install APK
adb install app/build/outputs/apk/debug/app-debug.apk
```

### Production Deployment

#### **Play Store Preparation**
1. **Create Release Keystore**
2. **Configure Signing**
3. **Build Release AAB**
4. **Upload to Play Console**
5. **Configure App Listing**
6. **Submit for Review**

#### **Direct Distribution**
1. **Build Release APK**
2. **Sign with Release Keystore**
3. **Distribute via Website/Email**
4. **Enable "Unknown Sources" on devices**

---

## 📱 **Quick Start Commands**

```bash
# 1. Navigate to project
cd apps/mobile/android-wallet

# 2. Clean and build debug APK
./gradlew clean assembleDebug

# 3. Install on connected device
adb install app/build/outputs/apk/debug/app-debug.apk

# 4. Build release APK (after setting up signing)
./gradlew assembleRelease
```

The APK will be ready for installation and testing! 🚀
