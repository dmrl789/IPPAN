# 🚀 Quick APK Setup Guide

## ⚡ **Fastest Way to Create APK**

### **Option 1: Android Studio (Recommended)**

1. **Download Android Studio**
   - Go to: https://developer.android.com/studio
   - Download and install (includes Java, Android SDK, build tools)

2. **Open Project**
   - Launch Android Studio
   - Open folder: `apps/mobile/android-wallet`
   - Wait for Gradle sync to complete

3. **Build APK**
   - Go to `Build` → `Build Bundle(s) / APK(s)` → `Build APK(s)`
   - Or use terminal: `./gradlew assembleDebug`

### **Option 2: Command Line Setup**

#### **Install Java 17**
```bash
# Windows (using Chocolatey)
choco install openjdk17

# Windows (using Scoop)
scoop install openjdk17

# macOS (using Homebrew)
brew install openjdk@17

# Ubuntu/Debian
sudo apt install openjdk-17-jdk
```

#### **Install Android SDK**
```bash
# Download Android Studio or SDK Command Line Tools
# Set environment variables:
export ANDROID_HOME=/path/to/android-sdk
export PATH=$PATH:$ANDROID_HOME/tools:$ANDROID_HOME/platform-tools
```

#### **Build APK**
```bash
cd apps/mobile/android-wallet
./gradlew assembleDebug
```

### **Option 3: Docker Build (No Local Setup)**

Create a Dockerfile for building without local setup:

```dockerfile
FROM openjdk:17-jdk-slim

# Install Android SDK
RUN apt-get update && apt-get install -y wget unzip
RUN wget https://dl.google.com/android/repository/commandlinetools-linux-9477386_latest.zip
RUN unzip commandlinetools-linux-9477386_latest.zip -d /opt/android-sdk
RUN mkdir -p /opt/android-sdk/cmdline-tools/latest
RUN mv /opt/android-sdk/cmdline-tools/* /opt/android-sdk/cmdline-tools/latest/

ENV ANDROID_HOME=/opt/android-sdk
ENV PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools

# Accept licenses and install platform
RUN yes | sdkmanager --licenses
RUN sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"

WORKDIR /app
COPY . .

# Build APK
RUN ./gradlew assembleDebug
```

## 📱 **APK Output Locations**

After successful build, you'll find:

- **Debug APK**: `app/build/outputs/apk/debug/app-debug.apk`
- **Release APK**: `app/build/outputs/apk/release/app-release.apk`
- **App Bundle**: `app/build/outputs/bundle/release/app-release.aab`

## 🔧 **Troubleshooting**

### **Common Issues & Solutions**

1. **"JAVA_HOME is not set"**
   ```bash
   # Windows
   set JAVA_HOME=C:\Program Files\Java\jdk-17
   
   # macOS/Linux
   export JAVA_HOME=/path/to/jdk-17
   ```

2. **"Android SDK not found"**
   ```bash
   # Set ANDROID_HOME
   export ANDROID_HOME=/path/to/android-sdk
   ```

3. **"Gradle build failed"**
   ```bash
   # Clean and rebuild
   ./gradlew clean assembleDebug
   ```

4. **"Permission denied" (Linux/macOS)**
   ```bash
   # Make gradlew executable
   chmod +x gradlew
   ```

## 🚀 **Quick Commands**

### **Windows**
```cmd
# Run the batch file
build-apk.bat

# Or manually
gradlew.bat assembleDebug
```

### **macOS/Linux**
```bash
# Make executable and run
chmod +x gradlew
./gradlew assembleDebug
```

### **Docker**
```bash
# Build with Docker
docker build -t ippan-wallet .
docker run -v $(pwd)/output:/app/build/outputs ippan-wallet
```

## 📦 **APK Features**

The built APK includes:

- ✅ **Complete Wallet Functionality**
- ✅ **Biometric Authentication**
- ✅ **QR Code Scanning**
- ✅ **Real-time Balance Updates**
- ✅ **Transaction History**
- ✅ **Multi-language Support**
- ✅ **Security Features**
- ✅ **Modern Material 3 UI**

## 🎯 **Next Steps After Building**

1. **Install APK**
   ```bash
   adb install app-debug.apk
   ```

2. **Test Features**
   - Create/import wallet
   - Send/receive tokens
   - Test biometric auth
   - Verify QR scanning

3. **Production Release**
   - Configure release signing
   - Build release APK
   - Upload to Play Store

---

## 💡 **Pro Tips**

- **Use Android Studio** for easiest setup
- **Debug APK** is perfect for testing
- **Release APK** requires signing configuration
- **App Bundle (AAB)** is preferred for Play Store
- **Test on real device** for best experience

The APK is ready to use! 🚀
