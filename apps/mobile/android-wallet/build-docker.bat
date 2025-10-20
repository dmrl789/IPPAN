@echo off
REM IPPAN Android Wallet - Docker Build Script for Windows

echo.
echo 🚀 IPPAN Android Wallet - Docker APK Builder
echo =============================================
echo.

REM Check if Docker is installed
docker --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Docker not found. Please install Docker first.
    echo    Download from: https://www.docker.com/get-started
    pause
    exit /b 1
)

echo ✅ Docker found
echo.

REM Create output directory
if not exist "output" mkdir output

echo 🐳 Building APK with Docker...
echo This may take a few minutes on first run...
echo.

REM Build Docker image
docker build -t ippan-wallet-builder .

if %errorlevel% neq 0 (
    echo ❌ Docker build failed!
    pause
    exit /b 1
)

echo ✅ Docker image built successfully
echo.

REM Run container and copy APK
echo 📱 Extracting APK...
docker run --rm -v "%cd%\output:/output" ippan-wallet-builder cp /app/app/build/outputs/apk/debug/app-debug.apk /output/

if %errorlevel% equ 0 (
    echo ✅ APK extracted successfully!
    echo.
    echo 📦 APK Location: output\app-debug.apk
    
    REM Check if APK exists and show info
    if exist "output\app-debug.apk" (
        echo 📏 APK Size: 
        dir "output\app-debug.apk" | find "app-debug.apk"
        echo.
        echo 🎉 APK is ready for installation!
        echo.
        echo 💡 Next steps:
        echo    1. Install APK on Android device: adb install output\app-debug.apk
        echo    2. Test all wallet features
        echo    3. Configure release signing for production
    )
) else (
    echo ❌ Failed to extract APK
    pause
    exit /b 1
)

echo.
pause
