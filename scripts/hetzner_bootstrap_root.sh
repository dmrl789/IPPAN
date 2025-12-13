set -euo pipefail

PUBKEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"

# 1) Ensure root can use SSH keys (so no more password after today)
install -d -m 700 /root/.ssh
grep -qF "$PUBKEY" /root/.ssh/authorized_keys 2>/dev/null || echo "$PUBKEY" >> /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys

# 2) Create ippan user + sudo + key auth
id -u ippan >/dev/null 2>&1 || useradd -m -s /bin/bash ippan
usermod -aG sudo ippan

install -d -m 700 /home/ippan/.ssh
echo "$PUBKEY" > /home/ippan/.ssh/authorized_keys
chmod 600 /home/ippan/.ssh/authorized_keys
chown -R ippan:ippan /home/ippan/.ssh

# 3) Passwordless sudo for automation (no password prompts ever)
echo "ippan ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/ippan
chmod 440 /etc/sudoers.d/ippan

echo "BOOTSTRAP_OK"

