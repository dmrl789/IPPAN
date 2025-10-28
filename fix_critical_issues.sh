#!/bin/bash

# IPPAN Critical Issues Fix Script
# This script addresses the most critical compilation issues

echo "🔧 Fixing IPPAN Critical Compilation Issues..."

# 1. Fix crypto crate NewAead imports
echo "📝 Fixing NewAead imports in crypto crate..."

# Fix encryption.rs
sed -i 's/use aes_gcm::aead::{Aead, NewAead};/use aes_gcm::aead::{Aead, KeyInit};/g' crates/crypto/src/encryption.rs
sed -i 's/use chacha20poly1305::aead::{Aead, NewAead};/use chacha20poly1305::aead::{Aead, KeyInit};/g' crates/crypto/src/encryption.rs

# Fix key_management.rs
sed -i 's/use aes_gcm::aead::{Aead, NewAead};/use aes_gcm::aead::{Aead, KeyInit};/g' crates/crypto/src/key_management.rs

# 2. Fix PBKDF2 usage
echo "📝 Fixing PBKDF2 usage..."
sed -i 's/use pbkdf2::{pbkdf2, Algorithm};/use pbkdf2::pbkdf2;/g' crates/crypto/src/encryption.rs
sed -i 's/pbkdf2(Algorithm::Sha256, password, salt, iterations, &mut key)/pbkdf2::<sha2::Sha256>(password, salt, iterations, &mut key)/g' crates/crypto/src/encryption.rs

# 3. Fix duplicate imports
echo "📝 Fixing duplicate imports..."
sed -i 's/use argon2::{Argon2, PasswordHasher};/use argon2::Argon2;/g' crates/crypto/src/encryption.rs
sed -i 's/use argon2::{Argon2, PasswordHasher, PasswordVerifier};/use argon2::Argon2;/g' crates/crypto/src/encryption.rs

# 4. Fix scrypt parameters
echo "📝 Fixing scrypt parameters..."
sed -i 's/let params = Params::new(log_n, r, p)/let params = Params::new(log_n, r, p, 32)/g' crates/crypto/src/key_management.rs

# 5. Remove unused imports from lib.rs
echo "📝 Removing unused imports..."
sed -i '/use serde::{Deserialize, Serialize};/d' crates/crypto/src/lib.rs
sed -i '/use std::collections::HashMap;/d' crates/crypto/src/lib.rs
sed -i '/use std::sync::Arc;/d' crates/crypto/src/lib.rs
sed -i '/use std::time::{Duration, Instant};/d' crates/crypto/src/lib.rs

# 6. Fix serde issues with Instant
echo "📝 Fixing serde issues with Instant..."
sed -i 's/#\[derive(Debug, Clone, Serialize, Deserialize)\]/#[derive(Debug, Clone)]/g' crates/crypto/src/key_management.rs

# 7. Fix base64 deprecated usage
echo "📝 Fixing deprecated base64 usage..."
sed -i 's/base64::encode(/base64::Engine::encode(\&base64::engine::general_purpose::STANDARD, /g' crates/crypto/src/hash_functions.rs
sed -i 's/base64::encode(/base64::Engine::encode(\&base64::engine::general_purpose::STANDARD, /g' crates/crypto/src/commitment_schemes.rs

echo "✅ Critical fixes applied!"
echo ""
echo "🔍 Testing compilation..."
cargo check -p ippan-crypto

if [ $? -eq 0 ]; then
    echo "✅ ippan-crypto compiles successfully!"
else
    echo "❌ Still have compilation errors. Check the output above."
fi

echo ""
echo "📋 Next steps:"
echo "1. Review the changes made"
echo "2. Test the crypto crate functionality"
echo "3. Fix any remaining compilation errors"
echo "4. Implement missing core functionality"
echo ""
echo "📖 See PRODUCTION_READINESS_AUDIT.md for complete assessment"
echo "📖 See CRITICAL_ISSUES_SUMMARY.md for detailed issues"