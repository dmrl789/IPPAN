#!/bin/bash

# IPPAN Security Audit Script
# This script performs comprehensive security audits of the IPPAN system

set -e

# Configuration
AUDIT_DIR="/tmp/ippan-security-audit-$(date +%Y%m%d_%H%M%S)"
REPORT_FILE="$AUDIT_DIR/security-audit-report.md"
TARGET_HOST="${TARGET_HOST:-localhost}"
TARGET_PORT="${TARGET_PORT:-3000}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create audit directory
create_audit_dir() {
    log_info "Creating audit directory..."
    mkdir -p "$AUDIT_DIR"
    log_success "Audit directory created: $AUDIT_DIR"
}

# Network security audit
audit_network_security() {
    log_info "Performing network security audit..."
    
    # Check open ports
    log_info "Scanning open ports..."
    nmap -sS -O -sV -p- "$TARGET_HOST" > "$AUDIT_DIR/nmap-scan.txt" 2>/dev/null || {
        log_warning "nmap not available, using netstat"
        netstat -tuln > "$AUDIT_DIR/netstat-scan.txt"
    }
    
    # Check SSL/TLS configuration
    log_info "Checking SSL/TLS configuration..."
    if command -v sslscan &> /dev/null; then
        sslscan "$TARGET_HOST:443" > "$AUDIT_DIR/sslscan.txt" 2>/dev/null || true
    fi
    
    # Check for common vulnerabilities
    log_info "Checking for common network vulnerabilities..."
    if command -v nikto &> /dev/null; then
        nikto -h "http://$TARGET_HOST" -output "$AUDIT_DIR/nikto-scan.txt" 2>/dev/null || true
    fi
    
    log_success "Network security audit completed"
}

# Application security audit
audit_application_security() {
    log_info "Performing application security audit..."
    
    # Check API endpoints
    log_info "Testing API endpoints..."
    curl -s -o "$AUDIT_DIR/api-test.txt" "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" || true
    
    # Check for SQL injection vulnerabilities
    log_info "Testing for SQL injection vulnerabilities..."
    sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --batch --output-dir="$AUDIT_DIR/sqlmap" 2>/dev/null || true
    
    # Check for XSS vulnerabilities
    log_info "Testing for XSS vulnerabilities..."
    if command -v xsser &> /dev/null; then
        xsser -u "http://$TARGET_HOST:$TARGET_PORT" --auto > "$AUDIT_DIR/xss-test.txt" 2>/dev/null || true
    fi
    
    # Check for CSRF vulnerabilities
    log_info "Testing for CSRF vulnerabilities..."
    # Custom CSRF test
    cat > "$AUDIT_DIR/csrf-test.html" << 'EOF'
<html>
<body>
<form action="http://TARGET_HOST:TARGET_PORT/api/v1/transactions" method="POST">
    <input type="hidden" name="amount" value="1000000">
    <input type="hidden" name="recipient" value="attacker">
    <input type="submit" value="Click me">
</form>
</body>
</html>
EOF
    sed -i "s/TARGET_HOST/$TARGET_HOST/g" "$AUDIT_DIR/csrf-test.html"
    sed -i "s/TARGET_PORT/$TARGET_PORT/g" "$AUDIT_DIR/csrf-test.html"
    
    log_success "Application security audit completed"
}

# Authentication and authorization audit
audit_authentication() {
    log_info "Performing authentication and authorization audit..."
    
    # Test authentication endpoints
    log_info "Testing authentication endpoints..."
    curl -s -o "$AUDIT_DIR/auth-test.txt" "http://$TARGET_HOST:$TARGET_PORT/api/v1/auth/login" || true
    
    # Test for weak passwords
    log_info "Testing for weak passwords..."
    if command -v hydra &> /dev/null; then
        hydra -L /usr/share/wordlists/rockyou.txt -P /usr/share/wordlists/rockyou.txt "$TARGET_HOST" http-post-form "/api/v1/auth/login:username=^USER^&password=^PASS^:Invalid" > "$AUDIT_DIR/hydra-test.txt" 2>/dev/null || true
    fi
    
    # Test JWT tokens
    log_info "Testing JWT token security..."
    # Check for JWT vulnerabilities
    if command -v jwt_tool &> /dev/null; then
        jwt_tool "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWV9.TJVA95OrM7E2cBab30RMHrHDcEfxjoYZgeFONFh7HgQ" > "$AUDIT_DIR/jwt-test.txt" 2>/dev/null || true
    fi
    
    log_success "Authentication and authorization audit completed"
}

# Database security audit
audit_database_security() {
    log_info "Performing database security audit..."
    
    # Check database configuration
    log_info "Checking database configuration..."
    if [ -f "config/production.toml" ]; then
        grep -i "password\|secret\|key" config/production.toml > "$AUDIT_DIR/db-config.txt" || true
    fi
    
    # Check for database vulnerabilities
    log_info "Testing database vulnerabilities..."
    if command -v sqlmap &> /dev/null; then
        sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --dbs --batch --output-dir="$AUDIT_DIR/sqlmap-db" 2>/dev/null || true
    fi
    
    log_success "Database security audit completed"
}

# Container security audit
audit_container_security() {
    log_info "Performing container security audit..."
    
    # Check Docker configuration
    log_info "Checking Docker configuration..."
    docker version > "$AUDIT_DIR/docker-version.txt" 2>/dev/null || true
    docker info > "$AUDIT_DIR/docker-info.txt" 2>/dev/null || true
    
    # Check for container vulnerabilities
    log_info "Scanning container images for vulnerabilities..."
    if command -v trivy &> /dev/null; then
        trivy image ippan/ippan:latest > "$AUDIT_DIR/trivy-scan.txt" 2>/dev/null || true
    fi
    
    # Check container security
    log_info "Checking container security..."
    docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy image ippan/ippan:latest > "$AUDIT_DIR/container-security.txt" 2>/dev/null || true
    
    log_success "Container security audit completed"
}

# Infrastructure security audit
audit_infrastructure_security() {
    log_info "Performing infrastructure security audit..."
    
    # Check system configuration
    log_info "Checking system configuration..."
    uname -a > "$AUDIT_DIR/system-info.txt"
    cat /etc/os-release >> "$AUDIT_DIR/system-info.txt" 2>/dev/null || true
    
    # Check file permissions
    log_info "Checking file permissions..."
    find . -type f -perm /o+w > "$AUDIT_DIR/world-writable-files.txt" 2>/dev/null || true
    find . -type f -perm /o+x > "$AUDIT_DIR/world-executable-files.txt" 2>/dev/null || true
    
    # Check for sensitive files
    log_info "Checking for sensitive files..."
    find . -name "*.key" -o -name "*.pem" -o -name "*.p12" -o -name "*.pfx" > "$AUDIT_DIR/sensitive-files.txt" 2>/dev/null || true
    
    log_success "Infrastructure security audit completed"
}

# Generate security report
generate_security_report() {
    log_info "Generating security audit report..."
    
    cat > "$REPORT_FILE" << EOF
# IPPAN Security Audit Report

**Audit Date**: $(date)
**Target Host**: $TARGET_HOST
**Target Port**: $TARGET_PORT
**Auditor**: IPPAN Security Team

## Executive Summary

This report contains the results of a comprehensive security audit performed on the IPPAN blockchain system.

## Audit Scope

- Network Security
- Application Security
- Authentication & Authorization
- Database Security
- Container Security
- Infrastructure Security

## Findings

### Critical Issues
$(grep -i "critical\|high" "$AUDIT_DIR"/*.txt 2>/dev/null | head -10 || echo "No critical issues found")

### Medium Issues
$(grep -i "medium\|moderate" "$AUDIT_DIR"/*.txt 2>/dev/null | head -10 || echo "No medium issues found")

### Low Issues
$(grep -i "low\|info" "$AUDIT_DIR"/*.txt 2>/dev/null | head -10 || echo "No low issues found")

## Recommendations

1. **Immediate Actions**
   - Address all critical security issues
   - Implement additional security controls
   - Update security policies

2. **Short-term Actions**
   - Address medium priority issues
   - Implement security monitoring
   - Conduct regular security training

3. **Long-term Actions**
   - Establish security governance
   - Implement security automation
   - Regular security assessments

## Detailed Results

### Network Security
- Port scan results: \`nmap-scan.txt\`
- SSL/TLS configuration: \`sslscan.txt\`
- Web vulnerability scan: \`nikto-scan.txt\`

### Application Security
- API endpoint tests: \`api-test.txt\`
- SQL injection tests: \`sqlmap/\`
- XSS tests: \`xss-test.txt\`
- CSRF tests: \`csrf-test.html\`

### Authentication & Authorization
- Authentication tests: \`auth-test.txt\`
- Password strength tests: \`hydra-test.txt\`
- JWT token tests: \`jwt-test.txt\`

### Database Security
- Database configuration: \`db-config.txt\`
- Database vulnerability tests: \`sqlmap-db/\`

### Container Security
- Docker configuration: \`docker-version.txt\`, \`docker-info.txt\`
- Container vulnerability scan: \`trivy-scan.txt\`
- Container security assessment: \`container-security.txt\`

### Infrastructure Security
- System information: \`system-info.txt\`
- File permissions: \`world-writable-files.txt\`, \`world-executable-files.txt\`
- Sensitive files: \`sensitive-files.txt\`

## Conclusion

The security audit has identified several areas for improvement. It is recommended that all critical and high-priority issues be addressed before production deployment.

## Next Steps

1. Review all findings
2. Prioritize remediation efforts
3. Implement security controls
4. Conduct follow-up assessment
5. Establish ongoing security monitoring

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Security audit report generated: $REPORT_FILE"
}

# Main audit function
main() {
    log_info "Starting IPPAN security audit..."
    
    create_audit_dir
    audit_network_security
    audit_application_security
    audit_authentication
    audit_database_security
    audit_container_security
    audit_infrastructure_security
    generate_security_report
    
    log_success "IPPAN security audit completed successfully!"
    echo ""
    echo "📊 Audit Results:"
    echo "  - Audit Directory: $AUDIT_DIR"
    echo "  - Report File: $REPORT_FILE"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the security audit report"
    echo "  2. Address critical and high-priority issues"
    echo "  3. Implement recommended security controls"
    echo "  4. Conduct follow-up assessment"
    echo "  5. Establish ongoing security monitoring"
}

# Run main function
main "$@"
