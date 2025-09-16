#!/bin/bash
# CI guard to fail if any mock/demo code exists
set -e

echo "🔍 Scanning for mock/demo code..."

if grep -RIn -E "(mock|demo|stub|fake|sample|faker)" src/ --exclude-dir=target; then
    echo "❌ MOCK FOUND - Build failed!"
    echo "Remove all mock/demo/stub/fake code before building real chain"
    exit 1
fi

echo "✅ No mocks found - Real mode approved"