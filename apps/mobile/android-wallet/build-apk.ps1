# IPPAN Android Wallet - APK Build Script
# PowerShell script to build APK for Windows

Write-Host "🚀 IPPAN Android Wallet - APK Builder" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

# Check if Java is installed
Write-Host "`n📋 Checking prerequisites..." -ForegroundColor Yellow

try {
    $javaVersion = java -version 2>&1
    Write-Host "✅ Java found: $($javaVersion[0])" -ForegroundColor Green
} catch {
    Write-Host "❌ Java not found. Please install JDK 17 first." -ForegroundColor Red
    Write-Host "   Download from: https://adoptium.net/temurin/releases/?version=17" -ForegroundColor Yellow
    Write-Host "   Or use: choco install openjdk17" -ForegroundColor Yellow
    exit 1
}

# Check if Android SDK is available
try {
    $androidHome = $env:ANDROID_HOME
    if (-not $androidHome) {
        $androidHome = "$env:USERPROFILE\AppData\Local\Android\Sdk"
    }
    
    if (Test-Path $androidHome) {
        Write-Host "✅ Android SDK found at: $androidHome" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Android SDK not found. Please install Android Studio or SDK." -ForegroundColor Yellow
        Write-Host "   Download from: https://developer.android.com/studio" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  Android SDK path not configured." -ForegroundColor Yellow
}

# Check if Gradle wrapper exists
if (Test-Path "gradlew.bat") {
    Write-Host "✅ Gradle wrapper found" -ForegroundColor Green
} else {
    Write-Host "❌ Gradle wrapper not found. Please run from project root." -ForegroundColor Red
    exit 1
}

Write-Host "`n🔧 Building APK..." -ForegroundColor Yellow

# Clean previous builds
Write-Host "🧹 Cleaning previous builds..." -ForegroundColor Blue
& .\gradlew.bat clean

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Clean failed. Please check your setup." -ForegroundColor Red
    exit 1
}

# Run tests
Write-Host "`n🧪 Running tests..." -ForegroundColor Blue
& .\gradlew.bat test

if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️  Some tests failed, but continuing with build..." -ForegroundColor Yellow
}

# Build debug APK
Write-Host "`n📱 Building debug APK..." -ForegroundColor Blue
& .\gradlew.bat assembleDebug

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ Debug APK built successfully!" -ForegroundColor Green
    
    $debugApk = "app\build\outputs\apk\debug\app-debug.apk"
    if (Test-Path $debugApk) {
        $apkSize = (Get-Item $debugApk).Length / 1MB
        Write-Host "📦 APK Location: $debugApk" -ForegroundColor Cyan
        Write-Host "📏 APK Size: $([math]::Round($apkSize, 2)) MB" -ForegroundColor Cyan
        
        # Check if ADB is available for installation
        try {
            $adbDevices = adb devices 2>&1
            if ($adbDevices -match "device$") {
                Write-Host "`n📱 Installing APK on connected device..." -ForegroundColor Blue
                adb install $debugApk
                if ($LASTEXITCODE -eq 0) {
                    Write-Host "✅ APK installed successfully!" -ForegroundColor Green
                } else {
                    Write-Host "⚠️  Installation failed. Check device connection." -ForegroundColor Yellow
                }
            } else {
                Write-Host "`n📱 No Android device connected for installation." -ForegroundColor Yellow
                Write-Host "   Connect a device with USB debugging enabled, or use an emulator." -ForegroundColor Yellow
            }
        } catch {
            Write-Host "`n📱 ADB not found. Install Android SDK platform-tools for device installation." -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "`n❌ Debug APK build failed!" -ForegroundColor Red
    Write-Host "   Check the error messages above for details." -ForegroundColor Yellow
    exit 1
}

# Try to build release APK (may fail without signing)
Write-Host "`n🔒 Attempting to build release APK..." -ForegroundColor Blue
& .\gradlew.bat assembleRelease

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Release APK built successfully!" -ForegroundColor Green
    
    $releaseApk = "app\build\outputs\apk\release\app-release.apk"
    if (Test-Path $releaseApk) {
        $apkSize = (Get-Item $releaseApk).Length / 1MB
        Write-Host "📦 Release APK Location: $releaseApk" -ForegroundColor Cyan
        Write-Host "📏 APK Size: $([math]::Round($apkSize, 2)) MB" -ForegroundColor Cyan
    }
} else {
    Write-Host "⚠️  Release APK build failed (likely due to signing configuration)." -ForegroundColor Yellow
    Write-Host "   This is normal for first-time builds. Debug APK is ready for testing." -ForegroundColor Yellow
}

Write-Host "`n🎉 Build process completed!" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "📱 Debug APK: app\build\outputs\apk\debug\app-debug.apk" -ForegroundColor Cyan
Write-Host "📱 Release APK: app\build\outputs\apk\release\app-release.apk" -ForegroundColor Cyan
Write-Host "`n💡 Next steps:" -ForegroundColor Yellow
Write-Host "   1. Install the APK on an Android device" -ForegroundColor White
Write-Host "   2. Test all wallet features" -ForegroundColor White
Write-Host "   3. Configure release signing for production" -ForegroundColor White
Write-Host "   4. Upload to Play Store when ready" -ForegroundColor White
