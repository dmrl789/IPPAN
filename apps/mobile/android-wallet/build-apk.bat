@echo off
REM IPPAN Android Wallet - APK Build Script
REM Simple batch file to build APK

echo.
echo 🚀 IPPAN Android Wallet - APK Builder
echo =====================================
echo.

REM Check if Java is available
java -version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Java not found. Please install JDK 17 first.
    echo    Download from: https://adoptium.net/temurin/releases/?version=17
    echo    Or use: choco install openjdk17
    pause
    exit /b 1
)

echo ✅ Java found
echo.

REM Check if Gradle wrapper exists
if not exist "gradlew.bat" (
    echo ❌ Gradle wrapper not found. Please run from project root.
    pause
    exit /b 1
)

echo ✅ Gradle wrapper found
echo.

REM Clean previous builds
echo 🧹 Cleaning previous builds...
call gradlew.bat clean
if %errorlevel% neq 0 (
    echo ❌ Clean failed. Please check your setup.
    pause
    exit /b 1
)

REM Build debug APK
echo.
echo 📱 Building debug APK...
call gradlew.bat assembleDebug
if %errorlevel% neq 0 (
    echo ❌ Debug APK build failed!
    echo    Check the error messages above for details.
    pause
    exit /b 1
)

echo.
echo ✅ Debug APK built successfully!
echo.

REM Check if APK was created
if exist "app\build\outputs\apk\debug\app-debug.apk" (
    echo 📦 APK Location: app\build\outputs\apk\debug\app-debug.apk
    echo.
    
    REM Try to install on connected device
    adb devices >nul 2>&1
    if %errorlevel% equ 0 (
        echo 📱 Installing APK on connected device...
        adb install app\build\outputs\apk\debug\app-debug.apk
        if %errorlevel% equ 0 (
            echo ✅ APK installed successfully!
        ) else (
            echo ⚠️  Installation failed. Check device connection.
        )
    ) else (
        echo 📱 No Android device connected for installation.
        echo    Connect a device with USB debugging enabled, or use an emulator.
    )
) else (
    echo ❌ APK not found. Build may have failed.
)

echo.
echo 🎉 Build process completed!
echo =====================================
echo 📱 Debug APK: app\build\outputs\apk\debug\app-debug.apk
echo.
echo 💡 Next steps:
echo    1. Install the APK on an Android device
echo    2. Test all wallet features
echo    3. Configure release signing for production
echo    4. Upload to Play Store when ready
echo.
pause
