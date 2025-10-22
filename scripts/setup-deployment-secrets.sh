#!/bin/bash

# IPPAN Deployment Secrets Setup Script
# This script helps configure GitHub secrets for automated deployment

set -e

echo "🔐 IPPAN Deployment Secrets Setup"
echo "=================================="
echo ""

# Check if GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) is not installed."
    echo "   Install it from: https://cli.github.com/"
    exit 1
fi

# Check if user is authenticated
if ! gh auth status &> /dev/null; then
    echo "❌ Not authenticated with GitHub CLI."
    echo "   Run: gh auth login"
    exit 1
fi

echo "✅ GitHub CLI is installed and authenticated"
echo ""

# Get repository information
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
echo "📦 Repository: $REPO"
echo ""

# Function to set secret
set_secret() {
    local name=$1
    local description=$2
    local example=$3
    
    echo "🔑 Setting secret: $name"
    echo "   Description: $description"
    if [ -n "$example" ]; then
        echo "   Example: $example"
    fi
    echo ""
    
    read -p "   Enter value (input will be hidden): " -s value
    echo ""
    
    if [ -z "$value" ]; then
        echo "   ⚠️  Skipping empty value"
        echo ""
        return
    fi
    
    gh secret set "$name" --body "$value"
    echo "   ✅ Secret $name set successfully"
    echo ""
}

echo "📋 Required Secrets for Automated Deployment"
echo "============================================"
echo ""

# Server 1 secrets
echo "🖥️  Server 1 Configuration (Full-Stack)"
echo "----------------------------------------"
set_secret "SERVER1_HOST" "IP address of Server 1" "188.245.97.41"
set_secret "SERVER1_USER" "SSH username for Server 1" "root"
set_secret "SERVER1_SSH_KEY" "Private SSH key for Server 1" "-----BEGIN OPENSSH PRIVATE KEY-----..."

echo ""
echo "🖥️  Server 2 Configuration (Node-Only)"
echo "---------------------------------------"
set_secret "SERVER2_HOST" "IP address of Server 2" "135.181.145.174"
set_secret "SERVER2_USER" "SSH username for Server 2" "root"
set_secret "SERVER2_SSH_KEY" "Private SSH key for Server 2" "-----BEGIN OPENSSH PRIVATE KEY-----..."

echo ""
echo "🎉 Deployment secrets setup complete!"
echo ""
echo "📖 Next Steps:"
echo "   1. Commit and push your changes to trigger deployment"
echo "   2. Monitor deployment in GitHub Actions tab"
echo "   3. Check server health after deployment"
echo ""
echo "📚 Documentation:"
echo "   - Automated Deployment Guide: docs/automated-deployment-guide.md"
echo "   - Troubleshooting: Check GitHub Actions logs"
echo ""
echo "🔗 Useful Commands:"
echo "   - View secrets: gh secret list"
echo "   - Update secret: gh secret set SECRET_NAME"
echo "   - Trigger deployment: git push origin main"
echo ""