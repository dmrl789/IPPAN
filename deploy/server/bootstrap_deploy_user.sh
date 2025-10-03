#!/usr/bin/env bash
# bootstrap_deploy_user.sh — one-time setup for safe /opt/ippan/ui deploys
# Usage (as root): bash bootstrap_deploy_user.sh "<GITHUB_ACTIONS_SSH_PUBLIC_KEY>"
# Example: sudo bash bootstrap_deploy_user.sh "ssh-ed25519 AAAAC3... ippan-ui-ci"

set -euo pipefail

DEPLOY_USER="deploy"
DEPLOY_GROUP="deploy"
DEPLOY_HOME="/home/${DEPLOY_USER}"
PUBKEY="${1:-}"

if [[ -z "${PUBKEY}" ]]; then
  echo "ERROR: provide the GitHub Actions SSH public key as the first argument."
  exit 1
fi

# 1) Ensure deploy user exists
if ! id -u "${DEPLOY_USER}" >/dev/null 2>&1; then
  adduser --disabled-password --gecos "" "${DEPLOY_USER}"
fi

# 2) SSH key for deploy user
install -d -m 700 "${DEPLOY_HOME}/.ssh"
touch "${DEPLOY_HOME}/.ssh/authorized_keys"
chmod 600 "${DEPLOY_HOME}/.ssh/authorized_keys"
if ! grep -qF "${PUBKEY}" "${DEPLOY_HOME}/.ssh/authorized_keys"; then
  echo "${PUBKEY}" >> "${DEPLOY_HOME}/.ssh/authorized_keys"
fi
chown -R "${DEPLOY_USER}:${DEPLOY_GROUP}" "${DEPLOY_HOME}/.ssh"

# 3) Target directories and ownership
install -d -o "${DEPLOY_USER}" -g "${DEPLOY_GROUP}" -m 755 /opt/ippan/ui
install -d -o "${DEPLOY_USER}" -g "${DEPLOY_GROUP}" -m 755 /opt/ippan/ui/releases
install -d -o "${DEPLOY_USER}" -g "${DEPLOY_GROUP}" -m 755 /opt/ippan/ui/tmp-upload

# 4) Minimal, scoped passwordless sudo
SUDO_FILE="/etc/sudoers.d/deploy-ippan"
cat > "${SUDO_FILE}" <<'EOC'
deploy ALL=(root) NOPASSWD: /usr/bin/install, /usr/bin/chown, /usr/bin/systemctl, /usr/bin/docker, /usr/bin/docker-compose, /usr/bin/docker*, /bin/mkdir, /bin/rm, /usr/bin/rsync, /bin/ln, /usr/sbin/nginx, /usr/bin/curl
EOC
chmod 440 "${SUDO_FILE}"
visudo -cf "${SUDO_FILE}" >/dev/null

echo "✅ Bootstrap completed."
echo "• Put the *private* key into the repo secret: SSH key -> DEPLOY_SSH_KEY"
echo "• Use user 'deploy' for GitHub Actions SSH steps."
