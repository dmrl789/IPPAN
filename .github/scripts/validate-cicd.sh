#!/bin/bash
set -euo pipefail

echo "🔍 Validating CI/CD Pipeline Configuration..."

# Check if all required workflows exist
REQUIRED_WORKFLOWS=(
  "ci.yml"
  "test.yml"
  "build.yml"
  "security-suite.yml"
  "release.yml"
  "check-nodes.yml"
  "prod-deploy.yml"
  "deploy-fix.yml"
  "dependabot.yml"
)

echo "📋 Checking required workflows..."
for workflow in "${REQUIRED_WORKFLOWS[@]}"; do
  if [[ -f ".github/workflows/${workflow}" ]]; then
    echo "✅ ${workflow} exists"
  else
    echo "❌ ${workflow} missing"
    exit 1
  fi
done

# Validate YAML syntax
echo "🔧 Validating YAML syntax..."
for workflow in .github/workflows/*.yml; do
  if command -v yamllint >/dev/null 2>&1; then
    yamllint "$workflow" || echo "⚠️ YAML linting issues in $workflow"
  else
    echo "ℹ️ yamllint not available, skipping YAML validation"
  fi
done

# Check for common issues
echo "🔍 Checking for common CI/CD issues..."

# Check for deprecated actions
echo "📦 Checking for deprecated actions..."
grep -r "actions/create-release@v1" .github/workflows/ && echo "❌ Found deprecated create-release@v1" || echo "✅ No deprecated create-release@v1"
grep -r "actions/upload-release-asset@v1" .github/workflows/ && echo "❌ Found deprecated upload-release-asset@v1" || echo "✅ No deprecated upload-release-asset@v1"

# Check for missing permissions
echo "🔐 Checking workflow permissions..."
for workflow in .github/workflows/*.yml; do
  if ! grep -q "permissions:" "$workflow"; then
    echo "⚠️ $workflow missing permissions section"
  fi
done

# Check for proper caching
echo "💾 Checking for proper caching..."
for workflow in .github/workflows/*.yml; do
  if grep -q "cargo" "$workflow" && ! grep -q "actions/cache" "$workflow"; then
    echo "⚠️ $workflow uses cargo but missing cache"
  fi
  if grep -q "npm" "$workflow" && ! grep -q "cache:" "$workflow"; then
    echo "⚠️ $workflow uses npm but missing cache"
  fi
done

# Check for security best practices
echo "🛡️ Checking security best practices..."
for workflow in .github/workflows/*.yml; do
  if grep -q "secrets.GITHUB_TOKEN" "$workflow" && ! grep -q "permissions:" "$workflow"; then
    echo "⚠️ $workflow uses GITHUB_TOKEN without explicit permissions"
  fi
done

echo "✅ CI/CD validation completed!"
