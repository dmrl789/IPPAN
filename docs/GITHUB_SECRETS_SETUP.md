# GitHub Secrets Setup Guide

This guide helps you configure the required GitHub secrets for CI/CD workflows in the IPPAN project.

## Quick Setup Checklist

### ✅ Required Secrets (Must Configure)

These secrets are **required** for deployment workflows to function:

- [ ] `DEPLOY_SSH_KEY` - SSH private key for server access
- [ ] `SERVER1_HOST` - Primary server IP/hostname (e.g., `188.245.97.41`)
- [ ] `SERVER1_USER` - SSH username for Server 1 (e.g., `root` or `ubuntu`)
- [ ] `SERVER2_HOST` - Secondary server IP/hostname (e.g., `135.181.145.174`)
- [ ] `SERVER2_USER` - SSH username for Server 2 (e.g., `root` or `ubuntu`)

### 📋 Optional Secrets (Recommended)

These secrets are optional but recommended for better security:

- [ ] `SERVER1_PORT` - SSH port for Server 1 (default: `22`)
- [ ] `SERVER1_SSH_KEY` - Server 1 specific key (falls back to `DEPLOY_SSH_KEY`)
- [ ] `SERVER1_FINGERPRINT` - SSH fingerprint for Server 1 verification
- [ ] `SERVER2_PORT` - SSH port for Server 2 (default: `22`)
- [ ] `SERVER2_SSH_KEY` - Server 2 specific key (falls back to `DEPLOY_SSH_KEY`)
- [ ] `SECONDARY_FINGERPRINT` - SSH fingerprint for Server 2 verification
- [ ] `NVD_API_KEY` - NIST NVD API key for security scanning

### 🔄 Fallback Aliases (Optional)

These are alternative names that some workflows support:

- [ ] `DEPLOY_HOST` - Alternative to `SERVER1_HOST`
- [ ] `DEPLOY_USER` - Alternative to `SERVER1_USER` / `SERVER2_USER`
- [ ] `DEPLOY_PORT` - Alternative to `SERVER1_PORT` / `SERVER2_PORT`
- [ ] `DEPLOY_FINGERPRINT` - Alternative to `SERVER1_FINGERPRINT`
- [ ] `SECONDARY_HOST` - Alternative to `SERVER2_HOST`
- [ ] `SECONDARY_PORT` - Alternative to `SERVER2_PORT`

---

## Step-by-Step Setup

### Step 1: Generate SSH Key

Generate a dedicated SSH key for deployments:

```bash
# Generate ED25519 key (recommended)
ssh-keygen -t ed25519 -C "ippan-ci-deployment" -f ~/.ssh/ippan_deploy_key

# Or generate RSA key (alternative)
ssh-keygen -t rsa -b 4096 -C "ippan-ci-deployment" -f ~/.ssh/ippan_deploy_key
```

This creates two files:
- `~/.ssh/ippan_deploy_key` - **Private key** (for GitHub Secret)
- `~/.ssh/ippan_deploy_key.pub` - **Public key** (for server)

### Step 2: Add Public Key to Servers

Copy the public key to each server:

```bash
# For Server 1
ssh-copy-id -i ~/.ssh/ippan_deploy_key.pub root@188.245.97.41

# For Server 2
ssh-copy-id -i ~/.ssh/ippan_deploy_key.pub root@135.181.145.174
```

Or manually:

```bash
# Copy public key content
cat ~/.ssh/ippan_deploy_key.pub

# SSH to server and append to authorized_keys
ssh root@188.245.97.41
mkdir -p ~/.ssh
echo "ssh-ed25519 AAAA... ippan-ci-deployment" >> ~/.ssh/authorized_keys
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
exit
```

### Step 3: Test SSH Connection

Verify the key works:

```bash
# Test Server 1
ssh -i ~/.ssh/ippan_deploy_key root@188.245.97.41 "echo 'Connection successful'"

# Test Server 2
ssh -i ~/.ssh/ippan_deploy_key root@135.181.145.174 "echo 'Connection successful'"
```

### Step 4: Add Secrets to GitHub

#### Using GitHub Web Interface:

1. Go to your repository on GitHub
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add each secret:

   **DEPLOY_SSH_KEY**:
   ```bash
   # Copy the ENTIRE private key including headers
   cat ~/.ssh/ippan_deploy_key
   ```
   Paste everything including:
   ```
   -----BEGIN OPENSSH PRIVATE KEY-----
   ...
   -----END OPENSSH PRIVATE KEY-----
   ```

   **SERVER1_HOST**:
   ```
   188.245.97.41
   ```

   **SERVER1_USER**:
   ```
   root
   ```

   **SERVER2_HOST**:
   ```
   135.181.145.174
   ```

   **SERVER2_USER**:
   ```
   root
   ```

#### Using GitHub CLI:

```bash
# Set secrets using gh CLI
gh secret set DEPLOY_SSH_KEY < ~/.ssh/ippan_deploy_key
gh secret set SERVER1_HOST -b "188.245.97.41"
gh secret set SERVER1_USER -b "root"
gh secret set SERVER2_HOST -b "135.181.145.174"
gh secret set SERVER2_USER -b "root"

# Optional: Set SSH ports if non-standard
gh secret set SERVER1_PORT -b "22"
gh secret set SERVER2_PORT -b "22"
```

### Step 5: Verify Secrets

List configured secrets:

```bash
gh secret list
```

Expected output:
```
DEPLOY_SSH_KEY      Updated 2025-11-11
SERVER1_HOST        Updated 2025-11-11
SERVER1_USER        Updated 2025-11-11
SERVER2_HOST        Updated 2025-11-11
SERVER2_USER        Updated 2025-11-11
```

### Step 6: Test Deployment Workflow

Trigger a manual deployment to test:

1. Go to **Actions** tab in GitHub
2. Select **Deploy IPPAN Full Stack** workflow
3. Click **Run workflow**
4. Select environment and options
5. Click **Run workflow** button
6. Monitor the workflow execution

---

## SSH Fingerprint Setup (Optional but Recommended)

Adding SSH fingerprints prevents man-in-the-middle attacks.

### Get Server Fingerprints

```bash
# For Server 1
ssh-keyscan -H 188.245.97.41 2>/dev/null | ssh-keygen -lf -

# For Server 2
ssh-keyscan -H 135.181.145.174 2>/dev/null | ssh-keygen -lf -
```

Example output:
```
256 SHA256:abc123... 188.245.97.41 (ED25519)
```

### Add to GitHub Secrets

```bash
gh secret set SERVER1_FINGERPRINT -b "SHA256:abc123..."
gh secret set SECONDARY_FINGERPRINT -b "SHA256:xyz789..."
```

---

## NVD API Key Setup (Optional)

For security vulnerability scanning in mobile builds:

1. Register at: https://nvd.nist.gov/developers/request-an-api-key
2. Check your email for the API key
3. Add to GitHub:

```bash
gh secret set NVD_API_KEY -b "your-nvd-api-key-here"
```

---

## Security Best Practices

### 1. Use Dedicated Deployment Keys

- **Never** use your personal SSH key for deployments
- Generate separate keys for CI/CD
- Limit key permissions on servers

### 2. Restrict SSH Key Access

On each server, restrict the deployment key:

```bash
# Edit authorized_keys
nano ~/.ssh/authorized_keys
```

Add restrictions before the key:
```
command="/opt/ippan/deploy.sh",no-port-forwarding,no-X11-forwarding,no-agent-forwarding ssh-ed25519 AAAA...
```

### 3. Use Non-Root Users (Recommended)

Instead of `root`, create a dedicated deployment user:

```bash
# On each server
sudo adduser ippan-deploy
sudo usermod -aG docker ippan-deploy
sudo mkdir -p /opt/ippan
sudo chown ippan-deploy:ippan-deploy /opt/ippan

# Update GitHub secrets
gh secret set SERVER1_USER -b "ippan-deploy"
gh secret set SERVER2_USER -b "ippan-deploy"
```

### 4. Rotate Keys Regularly

Rotate deployment keys every 90 days:

```bash
# Generate new key
ssh-keygen -t ed25519 -C "ippan-ci-$(date +%Y%m%d)" -f ~/.ssh/ippan_deploy_new

# Add to servers
ssh-copy-id -i ~/.ssh/ippan_deploy_new.pub root@188.245.97.41
ssh-copy-id -i ~/.ssh/ippan_deploy_new.pub root@135.181.145.174

# Update GitHub secret
gh secret set DEPLOY_SSH_KEY < ~/.ssh/ippan_deploy_new

# Test deployment

# Remove old key from servers
ssh root@188.245.97.41 "sed -i '/ippan-ci-deployment/d' ~/.ssh/authorized_keys"
ssh root@135.181.145.174 "sed -i '/ippan-ci-deployment/d' ~/.ssh/authorized_keys"
```

### 5. Monitor Secret Usage

Review GitHub Actions logs regularly:
- Check for failed authentication
- Monitor for unusual deployment patterns
- Set up alerts for failed deployments

### 6. Backup Secrets Securely

Store a backup of your deployment keys:
- Use a password manager (1Password, Bitwarden, etc.)
- Encrypt backups with GPG
- Store in a secure location

### 7. Principle of Least Privilege

Only grant necessary permissions:
- SSH keys should only access deployment directories
- Limit sudo access for deployment user
- Use read-only keys where possible

---

## Troubleshooting

### Problem: Workflow fails with "Missing secret"

**Solution**: Verify secret name matches exactly (case-sensitive):

```bash
gh secret list | grep -i server1_host
```

### Problem: SSH authentication fails

**Solution**: Test SSH connection manually:

```bash
# Use the same key file
ssh -i ~/.ssh/ippan_deploy_key -v root@188.245.97.41
```

Common issues:
- Private key not properly copied (must include `-----BEGIN/END-----`)
- Public key not in server's `authorized_keys`
- Wrong file permissions on server (should be `600` for `authorized_keys`)
- Firewall blocking SSH port

### Problem: "Permission denied (publickey)"

**Solution**: Check server-side:

```bash
# On server
sudo tail -f /var/log/auth.log  # Ubuntu/Debian
# or
sudo tail -f /var/log/secure     # CentOS/RHEL
```

### Problem: Workflow can't find host

**Solution**: Verify SERVER*_HOST values:

```bash
# Test DNS resolution
nslookup 188.245.97.41
ping 188.245.97.41
```

### Problem: "Host key verification failed"

**Solution**: Either add fingerprint or temporarily accept:

```bash
# Option 1: Add fingerprint (recommended)
ssh-keyscan -H 188.245.97.41 >> ~/.ssh/known_hosts

# Option 2: Workflows use StrictHostKeyChecking=no (less secure)
```

---

## Verification Checklist

Before considering setup complete, verify:

- [ ] All required secrets are set in GitHub
- [ ] SSH connection works from local machine using deployment key
- [ ] Deployment user has necessary permissions on servers
- [ ] Firewall allows SSH connections
- [ ] Docker and docker-compose are installed on servers
- [ ] `/opt/ippan` directory exists and is writable
- [ ] Git repository can be cloned on servers
- [ ] Test workflow completes successfully

---

## Quick Reference Commands

```bash
# Generate SSH key
ssh-keygen -t ed25519 -C "ippan-ci" -f ~/.ssh/ippan_deploy_key

# Copy to servers
ssh-copy-id -i ~/.ssh/ippan_deploy_key.pub root@188.245.97.41
ssh-copy-id -i ~/.ssh/ippan_deploy_key.pub root@135.181.145.174

# Set all required secrets at once
gh secret set DEPLOY_SSH_KEY < ~/.ssh/ippan_deploy_key
gh secret set SERVER1_HOST -b "188.245.97.41"
gh secret set SERVER1_USER -b "root"
gh secret set SERVER2_HOST -b "135.181.145.174"
gh secret set SERVER2_USER -b "root"

# Verify
gh secret list

# Test deployment
gh workflow run deploy-ippan-full-stack.yml
```

---

## Related Documentation

- [Secrets and Environment Variables](./SECRETS_AND_ENVIRONMENT_VARIABLES.md)
- [Deployment Instructions](../DEPLOYMENT_INSTRUCTIONS.md)
- [Full Stack Deployment Guide](../FULL_STACK_DEPLOYMENT_GUIDE.md)

---

**Last Updated**: 2025-11-11  
**Maintained By**: Agent-Theta, MetaAgent  
**Review Cycle**: Quarterly or after security incidents
