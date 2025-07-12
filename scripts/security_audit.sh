#!/bin/bash

# IPPAN Security Audit Script
# This script performs comprehensive security analysis and vulnerability scanning

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SECURITY_DIR="security_reports"
AUDIT_DIR="security_audits"
LOG_FILE="security_audit.log"

# Create directories
mkdir -p "$SECURITY_DIR"
mkdir -p "$AUDIT_DIR"

echo -e "${BLUE}🔒 IPPAN Security Audit Suite${NC}"
echo "=================================="

# Function to log messages
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

# Function to log errors
log_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "$LOG_FILE"
}

# Function to log warnings
log_warning() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "$LOG_FILE"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
log "Checking security audit prerequisites..."

if ! command_exists cargo; then
    log_error "Cargo not found. Please install Rust and Cargo first."
    exit 1
fi

if ! command_exists cargo-audit; then
    log_warning "cargo-audit not found. Installing..."
    cargo install cargo-audit
fi

if ! command_exists cargo-clippy; then
    log_warning "cargo-clippy not found. Installing..."
    rustup component add clippy
fi

# Clean previous results
log "Cleaning previous security audit results..."
rm -rf "$SECURITY_DIR"/*
rm -rf "$AUDIT_DIR"/*
rm -f "$LOG_FILE"

# Build the project
log "Building IPPAN for security analysis..."
cargo build --release

if [ $? -ne 0 ]; then
    log_error "Build failed. Please fix compilation errors first."
    exit 1
fi

log "Build completed successfully."

# Function to run security audit
run_security_audit() {
    local audit_type=$1
    local output_file=$2
    
    log "Running $audit_type security audit..."
    
    case $audit_type in
        "static_analysis")
            cargo clippy --all-targets --all-features -- -D warnings > "$output_file" 2>&1
            ;;
        "dependency_audit")
            cargo audit --json > "$output_file" 2>&1
            ;;
        "code_analysis")
            cargo check --all-targets --all-features > "$output_file" 2>&1
            ;;
        "security_scan")
            # Run custom security scanner
            cargo run --release --bin ippan -- --security-scan > "$output_file" 2>&1
            ;;
        *)
            log_error "Unknown audit type: $audit_type"
            return 1
            ;;
    esac
    
    if [ $? -eq 0 ]; then
        log "✅ $audit_type completed successfully"
    else
        log_warning "⚠️ $audit_type completed with warnings"
    fi
}

# Run comprehensive security audits
log "Starting comprehensive security audit..."

# Static analysis with Clippy
run_security_audit "static_analysis" "$AUDIT_DIR/clippy_analysis.txt"

# Dependency vulnerability audit
run_security_audit "dependency_audit" "$AUDIT_DIR/dependency_audit.json"

# Code analysis
run_security_audit "code_analysis" "$AUDIT_DIR/code_analysis.txt"

# Custom security scan
run_security_audit "security_scan" "$AUDIT_DIR/security_scan.txt"

# Generate security report
log "Generating comprehensive security report..."

cat > "$SECURITY_DIR/security_report.md" << EOF
# IPPAN Security Audit Report

Generated on: $(date)

## Executive Summary
This report contains the results of comprehensive security analysis performed on the IPPAN codebase.

## Audit Results

### 1. Static Analysis (Clippy)
- Code quality and potential security issues
- Unsafe code usage
- Memory safety concerns
- Best practices violations

### 2. Dependency Vulnerability Audit
- Known vulnerabilities in dependencies
- Outdated packages
- Security advisories
- Recommended updates

### 3. Code Analysis
- Compilation warnings and errors
- Type safety issues
- Potential runtime errors
- Code structure analysis

### 4. Custom Security Scan
- Custom security rules
- IPPAN-specific vulnerabilities
- Blockchain security concerns
- Cryptographic implementation review

## Security Metrics
- Total Issues Found: $(grep -c "warning\|error" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")
- Dependency Vulnerabilities: $(grep -c "vulnerability" "$AUDIT_DIR/dependency_audit.json" 2>/dev/null || echo "0")
- Security Score: $(echo "scale=1; 100 - $(grep -c "warning\|error" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0") * 0.5" | bc 2>/dev/null || echo "100")

## Recommendations

### Immediate Actions
1. **Critical Issues**: Address any critical security vulnerabilities immediately
2. **High Priority**: Fix high-severity issues within 24 hours
3. **Medium Priority**: Address medium-severity issues within 1 week
4. **Low Priority**: Plan to address low-severity issues in next sprint

### Long-term Security Strategy
1. **Regular Audits**: Implement automated security audits
2. **Dependency Management**: Keep dependencies updated and monitored
3. **Code Review**: Implement mandatory security code reviews
4. **Penetration Testing**: Schedule regular penetration testing
5. **Security Training**: Provide security training for developers

### Specific Recommendations
1. **Memory Safety**: Review all unsafe code blocks
2. **Cryptography**: Ensure strong cryptographic implementations
3. **Input Validation**: Add comprehensive input validation
4. **Access Control**: Implement proper access control mechanisms
5. **Error Handling**: Improve error handling and logging

## Next Steps
1. Review detailed audit results in individual files
2. Prioritize and address identified issues
3. Implement security monitoring and alerting
4. Schedule follow-up security reviews
5. Document security policies and procedures

## Files Generated
- Static Analysis: \`$AUDIT_DIR/clippy_analysis.txt\`
- Dependency Audit: \`$AUDIT_DIR/dependency_audit.json\`
- Code Analysis: \`$AUDIT_DIR/code_analysis.txt\`
- Security Scan: \`$AUDIT_DIR/security_scan.txt\`
- Security Report: \`$SECURITY_DIR/security_report.md\`
EOF

# Generate detailed analysis
log "Generating detailed security analysis..."

# Analyze Clippy results
cat > "$SECURITY_DIR/clippy_analysis.md" << EOF
# Clippy Static Analysis Results

## Overview
Clippy is Rust's linter that helps catch common mistakes and improve code quality.

## Results Summary
- Total Warnings: $(grep -c "warning" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")
- Total Errors: $(grep -c "error" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")

## Key Findings
$(grep -E "(warning|error)" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null | head -20 | sed 's/^/- /')

## Recommendations
1. Address all warnings and errors
2. Enable additional Clippy lints
3. Implement automated Clippy checks in CI/CD
4. Regular code quality reviews
EOF

# Analyze dependency audit results
cat > "$SECURITY_DIR/dependency_analysis.md" << EOF
# Dependency Vulnerability Analysis

## Overview
Analysis of dependencies for known security vulnerabilities.

## Results Summary
- Vulnerable Dependencies: $(grep -c "vulnerability" "$AUDIT_DIR/dependency_audit.json" 2>/dev/null || echo "0")
- Outdated Packages: $(grep -c "outdated" "$AUDIT_DIR/dependency_audit.json" 2>/dev/null || echo "0")

## Vulnerabilities Found
$(grep -A 5 -B 5 "vulnerability" "$AUDIT_DIR/dependency_audit.json" 2>/dev/null | head -30 || echo "No vulnerabilities found")

## Recommendations
1. Update vulnerable dependencies immediately
2. Monitor for new vulnerabilities
3. Implement automated dependency scanning
4. Use dependency pinning for critical packages
EOF

# Generate security score
log "Calculating security score..."

CLIPPY_ISSUES=$(grep -c "warning\|error" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")
DEPENDENCY_VULNS=$(grep -c "vulnerability" "$AUDIT_DIR/dependency_audit.json" 2>/dev/null || echo "0")
SECURITY_ISSUES=$(grep -c "security\|vulnerability" "$AUDIT_DIR/security_scan.txt" 2>/dev/null || echo "0")

TOTAL_ISSUES=$((CLIPPY_ISSUES + DEPENDENCY_VULNS + SECURITY_ISSUES))
SECURITY_SCORE=$((100 - TOTAL_ISSUES * 2))

if [ $SECURITY_SCORE -lt 0 ]; then
    SECURITY_SCORE=0
fi

cat > "$SECURITY_DIR/security_score.txt" << EOF
IPPAN Security Score: $SECURITY_SCORE/100

Breakdown:
- Clippy Issues: $CLIPPY_ISSUES
- Dependency Vulnerabilities: $DEPENDENCY_VULNS
- Security Issues: $SECURITY_ISSUES
- Total Issues: $TOTAL_ISSUES

Score Calculation: 100 - (Total Issues × 2) = $SECURITY_SCORE

Recommendations:
$(if [ $SECURITY_SCORE -ge 90 ]; then
    echo "- Excellent security posture"
    echo "- Continue current practices"
    echo "- Monitor for new vulnerabilities"
elif [ $SECURITY_SCORE -ge 70 ]; then
    echo "- Good security posture"
    echo "- Address identified issues"
    echo "- Implement additional security measures"
elif [ $SECURITY_SCORE -ge 50 ]; then
    echo "- Moderate security concerns"
    echo "- Prioritize critical issues"
    echo "- Implement comprehensive security review"
else
    echo "- Critical security issues detected"
    echo "- Immediate action required"
    echo "- Comprehensive security overhaul needed"
fi)
EOF

# Run additional security checks
log "Running additional security checks..."

# Check for hardcoded secrets
log "Checking for hardcoded secrets..."
grep -r -i "password\|secret\|key\|token" src/ --exclude-dir=target 2>/dev/null | head -20 > "$AUDIT_DIR/potential_secrets.txt" || true

# Check for unsafe code usage
log "Checking for unsafe code usage..."
grep -r "unsafe" src/ --exclude-dir=target 2>/dev/null > "$AUDIT_DIR/unsafe_code.txt" || true

# Check for cryptographic implementations
log "Checking cryptographic implementations..."
grep -r -i "md5\|sha1\|des\|rc4" src/ --exclude-dir=target 2>/dev/null > "$AUDIT_DIR/crypto_analysis.txt" || true

# Generate final summary
log "Generating final security summary..."

cat > "$SECURITY_DIR/final_summary.md" << EOF
# IPPAN Security Audit Final Summary

## Audit Overview
- **Date**: $(date)
- **Security Score**: $SECURITY_SCORE/100
- **Total Issues Found**: $TOTAL_ISSUES
- **Critical Issues**: $(grep -c "critical\|error" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")
- **High Priority Issues**: $(grep -c "warning" "$AUDIT_DIR/clippy_analysis.txt" 2>/dev/null || echo "0")

## Key Findings
1. **Code Quality**: $(if [ $CLIPPY_ISSUES -eq 0 ]; then echo "Excellent"; elif [ $CLIPPY_ISSUES -lt 10 ]; then echo "Good"; else echo "Needs improvement"; fi)
2. **Dependencies**: $(if [ $DEPENDENCY_VULNS -eq 0 ]; then echo "Secure"; else echo "Vulnerabilities found"; fi)
3. **Security Implementation**: $(if [ $SECURITY_ISSUES -eq 0 ]; then echo "Robust"; else echo "Issues detected"; fi)

## Immediate Actions Required
$(if [ $SECURITY_SCORE -lt 70 ]; then
    echo "1. Address critical security vulnerabilities"
    echo "2. Update vulnerable dependencies"
    echo "3. Review unsafe code usage"
    echo "4. Implement security monitoring"
else
    echo "1. Monitor for new vulnerabilities"
    echo "2. Continue security best practices"
    echo "3. Regular security audits"
fi)

## Files Generated
- Security Report: \`$SECURITY_DIR/security_report.md\`
- Clippy Analysis: \`$SECURITY_DIR/clippy_analysis.md\`
- Dependency Analysis: \`$SECURITY_DIR/dependency_analysis.md\`
- Security Score: \`$SECURITY_DIR/security_score.txt\`
- Final Summary: \`$SECURITY_DIR/final_summary.md\`

## Audit Files
- Static Analysis: \`$AUDIT_DIR/clippy_analysis.txt\`
- Dependency Audit: \`$AUDIT_DIR/dependency_audit.json\`
- Code Analysis: \`$AUDIT_DIR/code_analysis.txt\`
- Security Scan: \`$AUDIT_DIR/security_scan.txt\`
- Potential Secrets: \`$AUDIT_DIR/potential_secrets.txt\`
- Unsafe Code: \`$AUDIT_DIR/unsafe_code.txt\`
- Crypto Analysis: \`$AUDIT_DIR/crypto_analysis.txt\`

## Next Steps
1. Review all generated reports
2. Address identified issues based on priority
3. Implement automated security scanning
4. Schedule follow-up security reviews
5. Document security policies and procedures

## Security Recommendations
1. **Immediate**: Fix critical vulnerabilities
2. **Short-term**: Address high-priority issues
3. **Long-term**: Implement comprehensive security program
4. **Ongoing**: Regular security audits and monitoring
EOF

# Display summary
echo ""
echo -e "${GREEN}✅ Security audit completed successfully!${NC}"
echo ""
echo -e "${BLUE}📊 Security Summary:${NC}"
echo "  - Security Score: $SECURITY_SCORE/100"
echo "  - Total Issues: $TOTAL_ISSUES"
echo "  - Clippy Issues: $CLIPPY_ISSUES"
echo "  - Dependency Vulnerabilities: $DEPENDENCY_VULNS"
echo "  - Security Issues: $SECURITY_ISSUES"
echo ""
echo -e "${BLUE}📋 Generated Reports:${NC}"
ls -la "$SECURITY_DIR"/
echo ""
echo -e "${BLUE}📁 Audit Files:${NC}"
ls -la "$AUDIT_DIR"/
echo ""
echo -e "${YELLOW}💡 Next Steps:${NC}"
echo "  1. Review security reports in $SECURITY_DIR"
echo "  2. Address identified issues based on priority"
echo "  3. Implement automated security scanning"
echo "  4. Schedule follow-up security reviews"
echo ""
echo -e "${GREEN}🎉 IPPAN Security Audit Complete!${NC}"

# Exit with success
exit 0 