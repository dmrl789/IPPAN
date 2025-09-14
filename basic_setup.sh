# Simple deployment script
apt update && apt upgrade -y
apt install -y curl git wget unzip ufw fail2ban ca-certificates gnupg lsb-release
curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh && rm get-docker.sh
useradd -m -s /bin/bash -G sudo,docker ippan 2>/dev/null || true
mkdir -p /opt/ippan/mainnet /opt/ippan/data /opt/ippan/keys /opt/ippan/logs
chown -R ippan:ippan /opt/ippan
ufw allow 22/tcp && ufw allow 3000/tcp && ufw allow 8080/tcp && ufw allow 9090/tcp && ufw allow 3001/tcp
ufw --force enable
echo "Basic setup completed"
