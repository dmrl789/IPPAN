# SSH Key Setup for IPPAN Devnet-1

## Your SSH Public Key

Copy this key and add it to all 4 servers:

```
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan
```

---

## Manual Setup (Recommended)

### For each server, SSH as root and run:

```bash
# Create ippan user (if not exists)
if ! id -u ippan >/dev/null 2>&1; then
    useradd -m -s /bin/bash ippan
    usermod -aG sudo ippan
fi

# Set up SSH key
mkdir -p /home/ippan/.ssh
echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan" > /home/ippan/.ssh/authorized_keys
chmod 700 /home/ippan/.ssh
chmod 600 /home/ippan/.ssh/authorized_keys
chown -R ippan:ippan /home/ippan/.ssh

# Configure passwordless sudo
echo "ippan ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/ippan
chmod 440 /etc/sudoers.d/ippan
```

### Server List:

1. **node1** (Nürnberg): `ssh root@188.245.97.41`
   - Password: `vK3n9MKjWb9XtTsVAttP`

2. **node2** (Helsinki): `ssh root@135.181.145.174`
   - Password: `XhH7gUA7UM9gEPPALE7p`

3. **node3** (Singapore): `ssh root@5.223.51.238`
   - Password: `MriVKtEK9psU9RwMCidn`

4. **node4** (Ashburn): `ssh root@178.156.219.107`
   - Password: `hPAtPLw7hx3ndKXTW4vM`

---

## Quick One-Liner (Copy-paste for each server)

Replace `<SERVER_IP>` and `<PASSWORD>` with the values above:

```bash
ssh root@<SERVER_IP> "if ! id -u ippan >/dev/null 2>&1; then useradd -m -s /bin/bash ippan; usermod -aG sudo ippan; fi; mkdir -p /home/ippan/.ssh; echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEzegKgadJCRptuIM1aEVk/EaobuPAoMcssObcEO1uF+ ippan' > /home/ippan/.ssh/authorized_keys; chmod 700 /home/ippan/.ssh; chmod 600 /home/ippan/.ssh/authorized_keys; chown -R ippan:ippan /home/ippan/.ssh; echo 'ippan ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/ippan; chmod 440 /etc/sudoers.d/ippan"
```

---

## Verify Setup

After setting up all servers, test SSH access:

```powershell
ssh ippan@188.245.97.41 "whoami && sudo -n true && echo 'SSH and sudo OK'"
ssh ippan@135.181.145.174 "whoami && sudo -n true && echo 'SSH and sudo OK'"
ssh ippan@5.223.51.238 "whoami && sudo -n true && echo 'SSH and sudo OK'"
ssh ippan@178.156.219.107 "whoami && sudo -n true && echo 'SSH and sudo OK'"
```

All should return "ippan" and "SSH and sudo OK" without password prompts.

---

## After Setup

Once SSH keys are configured, run the automated deployment:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\devnet1_hetzner_autodeploy.ps1
```

