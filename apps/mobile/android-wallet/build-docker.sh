#!/bin/bash
# IPPAN Android Wallet - Docker Build Script

echo "🚀 IPPAN Android Wallet - Docker APK Builder"
echo "============================================="
echo

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    echo "   Download from: https://www.docker.com/get-started"
    exit 1
fi

echo "✅ Docker found"
echo

# Create output directory
mkdir -p output

echo "🐳 Building APK with Docker..."
echo "This may take a few minutes on first run..."
echo

# Build Docker image
docker build -t ippan-wallet-builder .

if [ $? -ne 0 ]; then
    echo "❌ Docker build failed!"
    exit 1
fi

echo "✅ Docker image built successfully"
echo

# Run container and copy APK
echo "📱 Extracting APK..."
docker run --rm -v $(pwd)/output:/output ippan-wallet-builder cp /app/app/build/outputs/apk/debug/app-debug.apk /output/

if [ $? -eq 0 ]; then
    echo "✅ APK extracted successfully!"
    echo
    echo "📦 APK Location: output/app-debug.apk"
    
    # Show APK info
    if [ -f "output/app-debug.apk" ]; then
        APK_SIZE=$(du -h output/app-debug.apk | cut -f1)
        echo "📏 APK Size: $APK_SIZE"
        echo
        echo "🎉 APK is ready for installation!"
        echo
        echo "💡 Next steps:"
        echo "   1. Install APK on Android device: adb install output/app-debug.apk"
        echo "   2. Test all wallet features"
        echo "   3. Configure release signing for production"
    fi
else
    echo "❌ Failed to extract APK"
    exit 1
fi
