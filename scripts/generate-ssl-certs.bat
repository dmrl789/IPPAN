@echo off
REM IPPAN SSL Certificate Generation Script for Windows
REM This script generates SSL certificates for production deployment

setlocal enabledelayedexpansion

REM Configuration
set CERT_DIR=.\deployments\ssl
set CA_KEY=%CERT_DIR%\ca.key
set CA_CERT=%CERT_DIR%\ca.crt
set SERVER_KEY=%CERT_DIR%\ippan.key
set SERVER_CERT=%CERT_DIR%\ippan.crt
set SERVER_CSR=%CERT_DIR%\ippan.csr

REM Create SSL directory
if not exist "%CERT_DIR%" mkdir "%CERT_DIR%"

echo 🔐 Generating SSL certificates for IPPAN production deployment...

REM Generate CA private key
echo 📝 Generating CA private key...
openssl genrsa -out "%CA_KEY%" 4096

REM Generate CA certificate
echo 📝 Generating CA certificate...
openssl req -new -x509 -days 3650 -key "%CA_KEY%" -out "%CA_CERT%" -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT Department/CN=IPPAN Root CA"

REM Generate server private key
echo 📝 Generating server private key...
openssl genrsa -out "%SERVER_KEY%" 4096

REM Generate server certificate signing request
echo 📝 Generating server certificate signing request...
openssl req -new -key "%SERVER_KEY%" -out "%SERVER_CSR%" -subj "/C=US/ST=CA/L=San Francisco/O=IPPAN/OU=IT Department/CN=ippan.network"

REM Create certificate extensions file
echo authorityKeyIdentifier=keyid,issuer > "%CERT_DIR%\server.ext"
echo basicConstraints=CA:FALSE >> "%CERT_DIR%\server.ext"
echo keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment >> "%CERT_DIR%\server.ext"
echo subjectAltName = @alt_names >> "%CERT_DIR%\server.ext"
echo. >> "%CERT_DIR%\server.ext"
echo [alt_names] >> "%CERT_DIR%\server.ext"
echo DNS.1 = ippan.network >> "%CERT_DIR%\server.ext"
echo DNS.2 = *.ippan.network >> "%CERT_DIR%\server.ext"
echo DNS.3 = localhost >> "%CERT_DIR%\server.ext"
echo IP.1 = 127.0.0.1 >> "%CERT_DIR%\server.ext"
echo IP.2 = ::1 >> "%CERT_DIR%\server.ext"

REM Generate server certificate
echo 📝 Generating server certificate...
openssl x509 -req -in "%SERVER_CSR%" -CA "%CA_CERT%" -CAkey "%CA_KEY%" -CAcreateserial -out "%SERVER_CERT%" -days 365 -extensions v3_req -extfile "%CERT_DIR%\server.ext"

REM Clean up temporary files
del "%SERVER_CSR%" "%CERT_DIR%\server.ext" "%CERT_DIR%\ca.srl" 2>nul

echo ✅ SSL certificates generated successfully!
echo 📁 Certificate files:
echo    - CA Certificate: %CA_CERT%
echo    - CA Private Key: %CA_KEY%
echo    - Server Certificate: %SERVER_CERT%
echo    - Server Private Key: %SERVER_KEY%
echo.
echo 🔒 Security Notes:
echo    - Keep private keys secure and never commit them to version control
echo    - Consider using a proper Certificate Authority for production
echo    - Rotate certificates regularly (current expiry: 365 days)
echo    - Use strong passphrases for production deployments

pause
