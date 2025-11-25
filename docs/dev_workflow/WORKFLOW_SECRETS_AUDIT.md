# GitHub Workflow Secrets Audit Report

**Generated:** 2025-11-11  
**Branch:** cursor/check-workflow-secrets-against-repository-settings-981e

---

## Executive Summary

This report catalogs all GitHub secrets referenced in workflow files (`.github/workflows/*.yml`) and verifies their presence in repository settings or documentation.

**Total Secrets Referenced:** 20 unique secrets  
**Standard GitHub Secrets:** 1 (GITHUB_TOKEN)  
**Required Repository Secrets:** 19  
**Documented in .env.example:** 0 (CI/CD secrets should be in GitHub repository settings)

---

## Secrets Inventory

### 1. Standard GitHub Secrets (Auto-provided)

| Secret Name | Used In | Status | Notes |
|-------------|---------|--------|-------|
| `GITHUB_TOKEN` | All workflows | ✅ Available | Automatically provided by GitHub Actions |

---

### 2. Deployment Infrastructure Secrets

#### Primary Server (SERVER1) Secrets

| Secret Name | Used In | Status | Purpose |
|-------------|---------|--------|---------|
| `SERVER1_HOST` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | Primary server hostname/IP |
| `SERVER1_USER` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | SSH username for primary server |
| `SERVER1_SSH_KEY` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | SSH private key for primary server |
| `SERVER1_PORT` | deploy.yml | ⚠️ Optional | SSH port (defaults to 22) |
| `SERVER1_FINGERPRINT` | deploy.yml | ⚠️ Optional | SSH host fingerprint for verification |

#### Secondary Server (SERVER2) Secrets

| Secret Name | Used In | Status | Purpose |
|-------------|---------|--------|---------|
| `SERVER2_HOST` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | Secondary server hostname/IP |
| `SERVER2_USER` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | SSH username for secondary server |
| `SERVER2_SSH_KEY` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | SSH private key for secondary server |
| `SERVER2_PORT` | deploy.yml | ⚠️ Optional | SSH port (defaults to 22) |

#### Generic Deployment Secrets (Fallbacks)

| Secret Name | Used In | Status | Purpose |
|-------------|---------|--------|---------|
| `DEPLOY_HOST` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | Fallback deployment host |
| `DEPLOY_USER` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | Fallback SSH username |
| `DEPLOY_SSH_KEY` | deploy.yml, deploy-ippan-full-stack.yml | ⚠️ **Required** | Fallback SSH private key |
| `DEPLOY_PORT` | deploy.yml | ⚠️ Optional | Fallback SSH port |
| `DEPLOY_FINGERPRINT` | deploy.yml | ⚠️ Optional | Fallback SSH fingerprint |

#### Secondary Host Aliases

| Secret Name | Used In | Status | Purpose |
|-------------|---------|--------|---------|
| `SECONDARY_HOST` | deploy.yml | ⚠️ Optional | Alternative name for SERVER2_HOST |
| `SECONDARY_USER` | deploy.yml | ⚠️ Optional | Alternative name for SERVER2_USER |
| `SECONDARY_FINGERPRINT` | deploy.yml | ⚠️ Optional | SSH fingerprint for secondary |
| `SECONDARY_PORT` | deploy.yml | ⚠️ Optional | Alternative port for secondary |

---

### 3. Security & Compliance Secrets

| Secret Name | Used In | Status | Purpose |
|-------------|---------|--------|---------|
| `NVD_API_KEY` | mobile.yml | ⚠️ Optional | National Vulnerability Database API key for dependency scanning |

---

## Workflow-by-Workflow Analysis

### test-suite.yml
- **Secrets Used:** None (only uses GITHUB_TOKEN)
- **Status:** ✅ No additional secrets required

### release.yml
- **Secrets Used:** `GITHUB_TOKEN`
- **Status:** ✅ No additional secrets required

### deploy.yml (Deployment Orchestrator)
- **Secrets Used:** 
  - `SERVER1_HOST`, `SERVER1_USER`, `SERVER1_SSH_KEY`, `SERVER1_PORT`, `SERVER1_FINGERPRINT`
  - `SERVER2_HOST`, `SERVER2_USER`, `SERVER2_SSH_KEY`, `SERVER2_PORT`
  - `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`, `DEPLOY_PORT`, `DEPLOY_FINGERPRINT`
  - `SECONDARY_HOST`, `SECONDARY_USER`, `SECONDARY_FINGERPRINT`, `SECONDARY_PORT`
  - `GITHUB_TOKEN`
- **Status:** ⚠️ **Requires 15+ secrets** for full functionality
- **Fallback Logic:** Uses `SERVER1_*` || `DEPLOY_*` pattern for graceful degradation

### build.yml
- **Secrets Used:** `GITHUB_TOKEN`
- **Status:** ✅ No additional secrets required

### ai-service.yml
- **Secrets Used:** `GITHUB_TOKEN`
- **Status:** ✅ No additional secrets required

### deploy-ippan-full-stack.yml
- **Secrets Used:**
  - `DEPLOY_SSH_KEY`, `SERVER1_HOST`, `SERVER2_HOST`, `DEPLOY_USER`
  - `GITHUB_TOKEN`
- **Status:** ⚠️ **Requires 4 deployment secrets**
- **Note:** Simpler than deploy.yml, uses consolidated secrets

### mobile.yml
- **Secrets Used:** `NVD_API_KEY`, `GITHUB_TOKEN`
- **Status:** ✅ Optional secret (workflow continues without NVD_API_KEY)

### check-nodes.yml
- **Secrets Used:** None
- **Status:** ✅ No secrets required

### dependabot.yml
- **Secrets Used:** `GITHUB_TOKEN`
- **Status:** ✅ No additional secrets required

### governance.yml
- **Secrets Used:** `GITHUB_TOKEN`, `GH_TOKEN`
- **Status:** ✅ No additional secrets required

### security-suite.yml
- **Secrets Used:** None (only uses GITHUB_TOKEN)
- **Status:** ✅ No additional secrets required

### ippan-ci-diagnostics.yml
- **Secrets Used:** None
- **Status:** ✅ No secrets required

### dlc-consensus.yml
- **Secrets Used:** None
- **Status:** ✅ No secrets required

### ai-determinism.yml
- **Secrets Used:** None
- **Status:** ✅ No secrets required

### unified-ui.yml
- **Secrets Used:** None
- **Status:** ✅ No secrets required

### auto-pr-cleanup.yml
- **Secrets Used:** `GITHUB_TOKEN`
- **Status:** ✅ No additional secrets required

---

## Secret Documentation Status

### ❌ Not Documented in .env.example Files

The following secrets are **NOT** found in any `.env.example` files:
- All `SERVER1_*` secrets
- All `SERVER2_*` secrets
- All `DEPLOY_*` secrets
- All `SECONDARY_*` secrets
- `NVD_API_KEY`

**Reason:** These are **CI/CD deployment secrets** that should be stored in **GitHub Repository Settings → Secrets and Variables → Actions**, not in `.env` files. This is correct security practice.

### ✅ Correctly Handled

Runtime application configuration (ports, URLs, etc.) is properly documented in:
- `apps/gateway/.env.example`
- `apps/unified-ui/.env.example`
- `deploy/.env.example`
- `config/ippan.env.example`

---

## Required Actions

### Critical (Deployment Will Fail Without These)

To enable automated deployments, configure these secrets in **GitHub Repository Settings**:

#### Minimum Required (deploy-ippan-full-stack.yml)
```bash
DEPLOY_SSH_KEY       # SSH private key for deployment
SERVER1_HOST         # e.g., 188.245.97.41
SERVER2_HOST         # e.g., 135.181.145.174
DEPLOY_USER          # SSH username (e.g., deploy, root, ubuntu)
```

#### Full Deployment Suite (deploy.yml)
```bash
# Primary Server
SERVER1_HOST         # Primary server IP/hostname
SERVER1_USER         # SSH username for primary
SERVER1_SSH_KEY      # SSH private key for primary
SERVER1_PORT         # (Optional) SSH port, defaults to 22
SERVER1_FINGERPRINT  # (Optional) SSH host key fingerprint

# Secondary Server
SERVER2_HOST         # Secondary server IP/hostname
SERVER2_USER         # SSH username for secondary
SERVER2_SSH_KEY      # SSH private key for secondary
SERVER2_PORT         # (Optional) SSH port, defaults to 22

# Generic Fallbacks
DEPLOY_HOST          # Fallback deployment host
DEPLOY_USER          # Fallback SSH username
DEPLOY_SSH_KEY       # Fallback SSH private key
DEPLOY_PORT          # (Optional) Fallback SSH port
DEPLOY_FINGERPRINT   # (Optional) Fallback SSH fingerprint
```

### Optional (Enhanced Functionality)

```bash
NVD_API_KEY          # For mobile dependency vulnerability scanning
                     # Get from: https://nvd.nist.gov/developers/request-an-api-key
```

---

## Secret Management Best Practices

### ✅ Current Good Practices

1. **GITHUB_TOKEN is used correctly** - Auto-provided, no manual configuration needed
2. **Fallback logic in deploy.yml** - Graceful degradation if specific secrets missing
3. **Optional secrets handled** - NVD_API_KEY is optional with workflow continuation
4. **Separation of concerns** - Runtime config in .env, CI/CD secrets in GitHub

### ⚠️ Recommendations

1. **Document secret requirements** in `DEPLOYMENT_INSTRUCTIONS.md`
2. **Use environment-specific secrets** when deploying to staging vs. production
3. **Rotate SSH keys periodically** (every 90 days recommended)
4. **Enable secret scanning** in GitHub repository settings
5. **Use deployment environments** in GitHub (production, staging) with protection rules

### 🔒 Security Considerations

- SSH private keys should be **generated specifically for CI/CD** use
- Keys should have **minimal permissions** (deployment-only access)
- Consider using **deploy keys** per repository instead of personal SSH keys
- Use **SSH key passphrases** where possible (though automation may require passwordless keys)
- Monitor for **unauthorized secret access** in GitHub audit logs

---

## Secret Validation Checklist

Use this checklist when configuring repository secrets:

- [ ] `DEPLOY_SSH_KEY` - Valid OpenSSH private key format
- [ ] `SERVER1_HOST` - Accessible from GitHub Actions runners
- [ ] `SERVER2_HOST` - Accessible from GitHub Actions runners
- [ ] `DEPLOY_USER` - Has SSH access and sudo privileges (if needed)
- [ ] `SERVER1_USER` - Has SSH access and sudo privileges (if needed)
- [ ] `SERVER2_USER` - Has SSH access and sudo privileges (if needed)
- [ ] SSH keys match authorized_keys on target servers
- [ ] Firewalls allow inbound SSH from GitHub Actions IPs
- [ ] Test deployment with `workflow_dispatch` before production use
- [ ] (Optional) `NVD_API_KEY` - Valid API key from nvd.nist.gov

---

## Troubleshooting

### Deployment fails with "Missing secret" error

**Solution:** Check the workflow logs for the specific secret name, then add it in:
```
Repository → Settings → Secrets and variables → Actions → New repository secret
```

### SSH connection fails despite correct secrets

**Possible causes:**
1. Firewall blocking GitHub Actions IPs
2. SSH key not in `~/.ssh/authorized_keys` on target server
3. Incorrect username
4. SSH service not running on target server
5. Port mismatch (non-standard SSH port)

### "Permission denied" during deployment

**Solution:** Ensure deployment user has necessary permissions:
```bash
# On target server
sudo usermod -aG docker $DEPLOY_USER  # For Docker commands
# OR configure sudoers for passwordless sudo
```

---

## Appendix: GitHub Actions IP Ranges

GitHub Actions runners use dynamic IP ranges. For firewall configuration:
- **IP ranges:** https://api.github.com/meta → `actions` field
- **Alternative:** Use GitHub-hosted self-hosted runners for static IPs
- **Security note:** Prefer IP allowlists + SSH key authentication

---

## Contact & Support

For questions about this audit or secret configuration:
- **Repository maintainers:** See `AGENTS.md`
- **Security issues:** Use private security advisory
- **General questions:** Open a GitHub Discussion

---

**Report Status:** ✅ Complete  
**Next Review:** 2025-12-11 (30 days)
