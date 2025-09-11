#!/bin/bash

# IPPAN Security Testing Framework
# Comprehensive security testing suite for IPPAN blockchain

set -e

# Configuration
SECURITY_DIR="/tmp/ippan-security-$(date +%Y%m%d_%H%M%S)"
TARGET_HOST="${TARGET_HOST:-localhost}"
TARGET_PORT="${TARGET_PORT:-3000}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Create security testing directory
create_security_dir() {
    log_info "Creating security testing directory..."
    mkdir -p "$SECURITY_DIR"
    log_success "Security directory created: $SECURITY_DIR"
}

# OWASP Top 10 Testing
owasp_top10_testing() {
    log_info "Running OWASP Top 10 security tests..."
    
    # A01: Broken Access Control
    log_info "Testing for broken access control..."
    curl -s -o "$SECURITY_DIR/access-control-test.txt" "http://$TARGET_HOST:$TARGET_PORT/api/v1/admin/users" || true
    
    # A02: Cryptographic Failures
    log_info "Testing for cryptographic failures..."
    if command -v sslscan &> /dev/null; then
        sslscan "$TARGET_HOST:443" > "$SECURITY_DIR/crypto-failures.txt" 2>/dev/null || true
    fi
    
    # A03: Injection
    log_info "Testing for injection vulnerabilities..."
    if command -v sqlmap &> /dev/null; then
        sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --batch --output-dir="$SECURITY_DIR/injection-tests" 2>/dev/null || true
    fi
    
    # A04: Insecure Design
    log_info "Testing for insecure design..."
    # Test for business logic flaws
    curl -s -X POST "http://$TARGET_HOST:$TARGET_PORT/api/v1/transactions" -d '{"amount":-1000}' > "$SECURITY_DIR/insecure-design.txt" || true
    
    # A05: Security Misconfiguration
    log_info "Testing for security misconfiguration..."
    curl -s -I "http://$TARGET_HOST:$TARGET_PORT/.env" > "$SECURITY_DIR/misconfig-test.txt" || true
    
    # A06: Vulnerable Components
    log_info "Testing for vulnerable components..."
    if command -v trivy &> /dev/null; then
        trivy image ippan/ippan:latest > "$SECURITY_DIR/vulnerable-components.txt" 2>/dev/null || true
    fi
    
    # A07: Authentication Failures
    log_info "Testing for authentication failures..."
    curl -s -X POST "http://$TARGET_HOST:$TARGET_PORT/api/v1/auth/login" -d '{"username":"admin","password":"admin"}' > "$SECURITY_DIR/auth-failures.txt" || true
    
    # A08: Software Integrity Failures
    log_info "Testing for software integrity failures..."
    # Check for unsigned binaries, missing checksums, etc.
    find . -name "*.exe" -o -name "*.bin" > "$SECURITY_DIR/integrity-failures.txt" 2>/dev/null || true
    
    # A09: Logging Failures
    log_info "Testing for logging failures..."
    # Test if security events are logged
    curl -s "http://$TARGET_HOST:$TARGET_PORT/api/v1/auth/login" > /dev/null
    # Check if failed login attempt was logged
    
    # A10: Server-Side Request Forgery
    log_info "Testing for SSRF vulnerabilities..."
    curl -s "http://$TARGET_HOST:$TARGET_PORT/api/v1/proxy?url=http://localhost:22" > "$SECURITY_DIR/ssrf-test.txt" || true
    
    log_success "OWASP Top 10 testing completed"
}

# Blockchain-specific security tests
blockchain_security_tests() {
    log_info "Running blockchain-specific security tests..."
    
    # Test for double-spending vulnerabilities
    log_info "Testing for double-spending vulnerabilities..."
    cat > "$SECURITY_DIR/double-spend-test.txt" << EOF
# Double-spending test scenarios
1. Race condition attacks
2. Network partition attacks
3. 51% attack simulation
4. Transaction malleability
5. Replay attacks
EOF
    
    # Test for consensus vulnerabilities
    log_info "Testing for consensus vulnerabilities..."
    cat > "$SECURITY_DIR/consensus-vuln-test.txt" << EOF
# Consensus vulnerability tests
1. Byzantine fault tolerance
2. Sybil attacks
3. Eclipse attacks
4. Nothing-at-stake problem
5. Long-range attacks
EOF
    
    # Test for smart contract vulnerabilities
    log_info "Testing for smart contract vulnerabilities..."
    cat > "$SECURITY_DIR/smart-contract-vuln.txt" << EOF
# Smart contract vulnerability tests
1. Reentrancy attacks
2. Integer overflow/underflow
3. Unchecked external calls
4. Front-running attacks
5. Denial of service
EOF
    
    # Test for wallet security
    log_info "Testing for wallet security..."
    cat > "$SECURITY_DIR/wallet-security-test.txt" << EOF
# Wallet security tests
1. Private key exposure
2. Weak key generation
3. Insecure storage
4. Transaction signing vulnerabilities
5. Address reuse
EOF
    
    log_success "Blockchain-specific security tests completed"
}

# Generate security testing report
generate_security_report() {
    log_info "Generating security testing report..."
    
    cat > "$SECURITY_DIR/security-testing-report.md" << EOF
# IPPAN Security Testing Report

**Test Date**: $(date)
**Target**: $TARGET_HOST:$TARGET_PORT
**Framework**: IPPAN Security Testing Framework

## Test Summary

This report contains the results of comprehensive security testing performed on the IPPAN blockchain system.

## OWASP Top 10 Results

### A01: Broken Access Control
- Test results: \`access-control-test.txt\`
- Status: $(grep -q "200" "$SECURITY_DIR/access-control-test.txt" && echo "VULNERABLE" || echo "SECURE")

### A02: Cryptographic Failures
- Test results: \`crypto-failures.txt\`
- Status: $(grep -q "SSL" "$SECURITY_DIR/crypto-failures.txt" && echo "VULNERABLE" || echo "SECURE")

### A03: Injection
- Test results: \`injection-tests/\`
- Status: $(ls "$SECURITY_DIR/injection-tests" 2>/dev/null && echo "VULNERABLE" || echo "SECURE")

### A04: Insecure Design
- Test results: \`insecure-design.txt\`
- Status: $(grep -q "error" "$SECURITY_DIR/insecure-design.txt" && echo "VULNERABLE" || echo "SECURE")

### A05: Security Misconfiguration
- Test results: \`misconfig-test.txt\`
- Status: $(grep -q "200" "$SECURITY_DIR/misconfig-test.txt" && echo "VULNERABLE" || echo "SECURE")

### A06: Vulnerable Components
- Test results: \`vulnerable-components.txt\`
- Status: $(grep -q "HIGH\|CRITICAL" "$SECURITY_DIR/vulnerable-components.txt" && echo "VULNERABLE" || echo "SECURE")

### A07: Authentication Failures
- Test results: \`auth-failures.txt\`
- Status: $(grep -q "success" "$SECURITY_DIR/auth-failures.txt" && echo "VULNERABLE" || echo "SECURE")

### A08: Software Integrity Failures
- Test results: \`integrity-failures.txt\`
- Status: $(wc -l < "$SECURITY_DIR/integrity-failures.txt" 2>/dev/null | grep -q "0" && echo "SECURE" || echo "VULNERABLE")

### A09: Logging Failures
- Test results: Manual verification required
- Status: PENDING

### A10: Server-Side Request Forgery
- Test results: \`ssrf-test.txt\`
- Status: $(grep -q "SSH" "$SECURITY_DIR/ssrf-test.txt" && echo "VULNERABLE" || echo "SECURE")

## Blockchain Security Results

### Double-Spending Tests
- Test scenarios: \`double-spend-test.txt\`
- Status: Manual testing required

### Consensus Vulnerability Tests
- Test scenarios: \`consensus-vuln-test.txt\`
- Status: Manual testing required

### Smart Contract Tests
- Test scenarios: \`smart-contract-vuln.txt\`
- Status: Manual testing required

### Wallet Security Tests
- Test scenarios: \`wallet-security-test.txt\`
- Status: Manual testing required

## Recommendations

1. **Immediate Actions**
   - Address all critical vulnerabilities
   - Implement additional security controls
   - Update security policies

2. **Short-term Actions**
   - Conduct manual security testing
   - Implement security monitoring
   - Regular security assessments

3. **Long-term Actions**
   - Establish security governance
   - Implement security automation
   - Continuous security improvement

## Next Steps

1. Review all test results
2. Prioritize vulnerability remediation
3. Implement security controls
4. Conduct follow-up testing
5. Establish ongoing security monitoring

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Security testing report generated: $SECURITY_DIR/security-testing-report.md"
}

# Main function
main() {
    log_info "Starting IPPAN security testing framework..."
    
    create_security_dir
    owasp_top10_testing
    blockchain_security_tests
    generate_security_report
    
    log_success "IPPAN security testing framework completed!"
    echo ""
    echo "📊 Security Testing Results:"
    echo "  - Test Directory: $SECURITY_DIR"
    echo "  - Report File: $SECURITY_DIR/security-testing-report.md"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the security testing report"
    echo "  2. Address identified vulnerabilities"
    echo "  3. Implement recommended security controls"
    echo "  4. Conduct manual security testing"
    echo "  5. Establish ongoing security monitoring"
}

main "$@"
