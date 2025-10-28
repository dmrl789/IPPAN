#!/bin/bash

# Fix remaining compilation issues in IPPAN workspace
# This script addresses the most critical compilation errors

echo "🔧 Fixing remaining compilation issues..."

# Fix ValidatorId Display implementation
echo "📝 Adding Display implementation for ValidatorId..."
cat > /tmp/validator_id_display.patch << 'EOF'
--- a/crates/types/src/lib.rs
+++ b/crates/types/src/lib.rs
@@ -1,3 +1,5 @@
+use std::fmt;
+
 //! IPPAN Types
 //!
 //! This crate defines the core types used throughout the IPPAN blockchain.
@@ -50,6 +52,12 @@ pub struct ValidatorId {
     pub bytes: [u8; 32],
 }
 
+impl fmt::Display for ValidatorId {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        write!(f, "{:?}", self)
+    }
+}
+
 /// Block hash type
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
 pub struct BlockHash {
EOF

# Apply the patch if the file exists
if [ -f "crates/types/src/lib.rs" ]; then
    patch -p1 < /tmp/validator_id_display.patch
    echo "✅ Added Display implementation for ValidatorId"
else
    echo "⚠️  types crate not found, skipping ValidatorId fix"
fi

# Fix AI Core error variants
echo "📝 Adding missing error variants to AiCoreError..."
if [ -f "crates/ai_core/src/errors.rs" ]; then
    # Add ExecutionFailed variant if it doesn't exist
    if ! grep -q "ExecutionFailed" "crates/ai_core/src/errors.rs"; then
        sed -i '/pub enum AiCoreError {/a\    ExecutionFailed(String),' "crates/ai_core/src/errors.rs"
        echo "✅ Added ExecutionFailed variant to AiCoreError"
    else
        echo "✅ ExecutionFailed variant already exists"
    fi
else
    echo "⚠️  AI Core errors.rs not found, skipping error variant fix"
fi

# Fix base64 usage in AI Core
echo "📝 Fixing deprecated base64 usage..."
if [ -f "crates/ai_core/src/models.rs" ]; then
    # Replace deprecated base64::decode with Engine::decode
    sed -i 's/base64::decode(/base64::Engine::decode(\&base64::engine::general_purpose::STANDARD, /g' "crates/ai_core/src/models.rs"
    echo "✅ Fixed deprecated base64 usage"
else
    echo "⚠️  AI Core models.rs not found, skipping base64 fix"
fi

# Clean up temporary files
rm -f /tmp/validator_id_display.patch

echo "🎉 Compilation fixes applied!"
echo "📋 Next steps:"
echo "   1. Run 'cargo check' to verify fixes"
echo "   2. Fix remaining field access errors"
echo "   3. Add missing types and structs"
echo "   4. Test full workspace compilation"