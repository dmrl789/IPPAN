set -euo pipefail

PUBKEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan"
USERNAME="ippan-devnet"

# 1) Ensure root can use SSH keys
install -d -m 700 /root/.ssh
grep -qF "$PUBKEY" /root/.ssh/authorized_keys 2>/dev/null || echo "$PUBKEY" >> /root/.ssh/authorized_keys
chmod 600 /root/.ssh/authorized_keys

# 2) Create ippan-devnet user + sudo + key auth
id -u $USERNAME >/dev/null 2>&1 || useradd -m -s /bin/bash $USERNAME
usermod -aG sudo $USERNAME

install -d -m 700 /home/$USERNAME/.ssh
echo "$PUBKEY" > /home/$USERNAME/.ssh/authorized_keys
chmod 600 /home/$USERNAME/.ssh/authorized_keys
chown -R $USERNAME:$USERNAME /home/$USERNAME/.ssh

# 3) Passwordless sudo for automation
echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME
chmod 440 /etc/sudoers.d/$USERNAME

echo "BOOTSTRAP_OK"

