# IPPAN Full Stack Deployment

This directory contains the complete deployment setup for the IPPAN blockchain full-stack architecture across two servers.

## 📁 Structure

```
deploy/
├── docker-compose.full-stack.yml    # Server 1: UI + Gateway + Node 1 + Nginx
├── docker-compose.production.yml    # Server 2: Node 2 only
├── nginx/
│   └── load-balancer.conf           # Nginx load balancer configuration
├── .env.example                     # Environment variables template
├── deploy-server1.sh                # Deploy script for Server 1
├── deploy-server2.sh                # Deploy script for Server 2
├── verify-deployment.sh             # Verification script for both servers
└── README.md                        # This file
```

## 🖥️ Server Architecture

### Server 1 (188.245.97.41) - Full Stack
- **IPPAN UI** (Port 3001)
- **IPPAN Gateway** (Port 8081)
- **IPPAN Node 1** (Port 8080, P2P 4001)
- **Nginx Load Balancer** (Ports 80, 443)

### Server 2 (135.181.145.174) - Node Only
- **IPPAN Node 2** (Port 8080, P2P 4001)

## 🚀 Quick Deployment

### Prerequisites

1. **Docker & Docker Compose** installed on both servers
2. **Firewall ports** opened:
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   sudo ufw allow 8080/tcp
   sudo ufw allow 4001/tcp
   sudo ufw reload
   ```

### Deploy Server 1 (Full Stack)

```bash
cd deploy
chmod +x deploy-server1.sh
./deploy-server1.sh
```

### Deploy Server 2 (Node 2)

```bash
cd deploy
chmod +x deploy-server2.sh
./deploy-server2.sh
```

### Verify Deployment

```bash
chmod +x verify-deployment.sh
./verify-deployment.sh
```

## 🐳 Manual Docker Commands

### Server 1 Commands

```bash
# Start full stack
docker-compose -f docker-compose.full-stack.yml up -d

# View logs
docker-compose -f docker-compose.full-stack.yml logs -f

# Check status
docker-compose -f docker-compose.full-stack.yml ps

# Stop services
docker-compose -f docker-compose.full-stack.yml down
```

### Server 2 Commands

```bash
# Start Node 2
docker-compose -f docker-compose.production.yml up -d

# View logs
docker-compose -f docker-compose.production.yml logs -f

# Check status
docker-compose -f docker-compose.production.yml ps

# Stop services
docker-compose -f docker-compose.production.yml down
```

## 🔍 Health Checks

### Automated Verification

```bash
./verify-deployment.sh
```

### Manual Health Checks

#### Server 1 Endpoints
```bash
# UI Frontend
curl http://188.245.97.41:3001/

# Node 1 RPC
curl http://188.245.97.41:8080/health

# Gateway
curl http://188.245.97.41:8081/

# Nginx Load Balancer
curl http://188.245.97.41:80/
```

#### Server 2 Endpoints
```bash
# Node 2 RPC
curl http://135.181.145.174:8080/health

# P2P Port Check
ss -ltnp | grep :4001
```

## 🔧 Configuration

### Environment Variables

Copy and customize the environment template:

```bash
cp .env.example .env
# Edit .env with your specific values
```

### Nginx SSL Configuration

To enable SSL/TLS:

1. Obtain SSL certificates (Let's Encrypt recommended)
2. Update `nginx/load-balancer.conf` with your certificate paths
3. Restart the nginx container

## 🔄 CI/CD Deployment

The repository includes a GitHub Actions workflow (`.github/workflows/deploy-ippan-full-stack.yml`) that automatically deploys to both servers when changes are pushed to the `main` branch.

### Required Secrets

Set these in your GitHub repository settings:

```
DEPLOY_SSH_KEY      # Private SSH key for server access
DEPLOY_USER         # SSH username (e.g., ubuntu)
SERVER1_HOST        # 188.245.97.41
SERVER2_HOST        # 135.181.145.174
DEPLOY_APP_DIR      # Deployment directory (optional, defaults to ~/ippan-deploy)
```

### Manual Workflow Trigger

You can manually trigger deployment from GitHub Actions with options for:
- Environment selection (production/staging)
- Force rebuild of Docker images

## 🛠️ Troubleshooting

### Common Issues

1. **Port conflicts**: Ensure no other services are using ports 3001, 8080, 8081, 4001
2. **Docker permissions**: Add user to docker group: `sudo usermod -aG docker $USER`
3. **Firewall blocking**: Verify UFW rules and cloud provider security groups
4. **Container startup failures**: Check logs with `docker-compose logs -f`

### Debug Commands

```bash
# Check port usage
ss -ltnp | grep -E ':3001|:8080|:8081|:4001'

# Check Docker containers
docker ps -a

# Check Docker networks
docker network ls

# Check system resources
df -h
free -m
```

### Recovery Commands

```bash
# Force restart all services
docker-compose -f docker-compose.full-stack.yml down --remove-orphans
docker-compose -f docker-compose.full-stack.yml up -d --force-recreate

# Clean up Docker resources
docker system prune -f
docker volume prune -f
```

## 📊 Monitoring

### Service Status

```bash
# Check all containers
docker-compose -f docker-compose.full-stack.yml ps

# Monitor logs in real-time
docker-compose -f docker-compose.full-stack.yml logs -f --tail=100

# Check resource usage
docker stats
```

### Network Connectivity

```bash
# Test P2P connectivity between nodes
nc -zv 135.181.145.174 4001  # From Server 1
nc -zv 188.245.97.41 4001    # From Server 2
```

## 🔐 Security Considerations

1. **Firewall**: Only open necessary ports
2. **SSL/TLS**: Configure proper certificates for production
3. **Docker security**: Keep images updated and scan for vulnerabilities
4. **SSH access**: Use key-based authentication only
5. **Network isolation**: Consider using Docker networks for internal communication

## 📞 Support

For issues or questions:
1. Check the logs: `docker-compose logs -f`
2. Run verification: `./verify-deployment.sh`
3. Review this README and troubleshooting section
4. Check the main project documentation

---

**Last Updated**: $(date)
**Version**: 1.0.0