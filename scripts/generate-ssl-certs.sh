#!/bin/bash

# IPPAN SSL Certificate Generation Script
# This script generates SSL certificates for production deployment

set -e

# Configuration
CERT_DIR="./deployments/ssl"
CA_KEY="$CERT_DIR/ca.key"
CA_CERT="$CERT_DIR/ca.crt"
SERVER_KEY="$CERT_DIR/ippan.key"
SERVER_CERT="$CERT_DIR/ippan.crt"
SERVER_CSR="$CERT_DIR/ippan.csr"

# Create SSL directory
mkdir -p "$CERT_DIR"

echo "🔐 Generating SSL certificates for IPPAN production deployment..."

# Generate CA private key
echo "📝 Generating CA private key..."
openssl genrsa -out "$CA_KEY" 4096

# Generate CA certificate
echo "📝 Generating CA certificate..."
openssl req -new -x509 -days 3650 -key "$CA_KEY" -out "$CA_CERT" \
    -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT Department/CN=IPPAN Root CA"

# Generate server private key
echo "📝 Generating server private key..."
openssl genrsa -out "$SERVER_KEY" 4096

# Generate server certificate signing request
echo "📝 Generating server certificate signing request..."
openssl req -new -key "$SERVER_KEY" -out "$SERVER_CSR" \
    -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT Department/CN=ippan.network"

# Create certificate extensions file
cat > "$CERT_DIR/server.ext" << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = ippan.network
DNS.2 = *.ippan.network
DNS.3 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Generate server certificate
echo "📝 Generating server certificate..."
openssl x509 -req -in "$SERVER_CSR" -CA "$CA_CERT" -CAkey "$CA_KEY" \
    -CAcreateserial -out "$SERVER_CERT" -days 365 \
    -extensions v3_req -extfile "$CERT_DIR/server.ext"

# Set proper permissions
chmod 600 "$CA_KEY" "$SERVER_KEY"
chmod 644 "$CA_CERT" "$SERVER_CERT"

# Clean up temporary files
rm -f "$SERVER_CSR" "$CERT_DIR/server.ext" "$CERT_DIR/ca.srl"

echo "✅ SSL certificates generated successfully!"
echo "📁 Certificate files:"
echo "   - CA Certificate: $CA_CERT"
echo "   - CA Private Key: $CA_KEY"
echo "   - Server Certificate: $SERVER_CERT"
echo "   - Server Private Key: $SERVER_KEY"
echo ""
echo "🔒 Security Notes:"
echo "   - Keep private keys secure and never commit them to version control"
echo "   - Consider using a proper Certificate Authority for production"
echo "   - Rotate certificates regularly (current expiry: 365 days)"
echo "   - Use strong passphrases for production deployments"
