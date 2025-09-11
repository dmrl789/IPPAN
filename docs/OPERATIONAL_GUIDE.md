# IPPAN Blockchain Operational Guide

## Table of Contents
1. [Overview](#overview)
2. [Node Operations](#node-operations)
3. [Network Management](#network-management)
4. [Consensus Operations](#consensus-operations)
5. [Transaction Management](#transaction-management)
6. [Wallet Operations](#wallet-operations)
7. [Monitoring and Alerting](#monitoring-and-alerting)
8. [Performance Optimization](#performance-optimization)
9. [Security Operations](#security-operations)
10. [Backup and Recovery](#backup-and-recovery)
11. [Troubleshooting](#troubleshooting)
12. [Maintenance Procedures](#maintenance-procedures)

## Overview

This operational guide provides comprehensive instructions for operating and maintaining the IPPAN blockchain network in production environments. It covers day-to-day operations, monitoring, troubleshooting, and maintenance procedures.

## Node Operations

### 1. Starting and Stopping Nodes

#### Starting a Node
```bash
# Start node with configuration
./target/release/ippan-node --config /etc/ippan/configs/production-node.toml

# Start as systemd service
sudo systemctl start ippan

# Start with Docker
docker run -d --name ippan-node ippan:latest

# Start with Kubernetes
kubectl apply -f deployments/kubernetes/ippan-deployment.yaml
```

#### Stopping a Node
```bash
# Graceful shutdown
sudo systemctl stop ippan

# Force stop
sudo systemctl kill ippan

# Docker stop
docker stop ippan-node

# Kubernetes stop
kubectl delete -f deployments/kubernetes/ippan-deployment.yaml
```

#### Restarting a Node
```bash
# Restart service
sudo systemctl restart ippan

# Restart with Docker
docker restart ippan-node

# Rolling restart in Kubernetes
kubectl rollout restart deployment/ippan-node
```

### 2. Node Status and Health Checks

#### Check Node Status
```bash
# System service status
sudo systemctl status ippan

# Docker container status
docker ps | grep ippan

# Kubernetes pod status
kubectl get pods -l app=ippan

# Node health via API
curl http://localhost:8080/health
```

#### Health Check Endpoints
```bash
# Overall health
curl http://localhost:8080/health

# Component health
curl http://localhost:8080/health/consensus
curl http://localhost:8080/health/network
curl http://localhost:8080/health/storage
curl http://localhost:8080/health/database

# Detailed status
curl http://localhost:8080/status
```

### 3. Node Configuration Management

#### Update Configuration
```bash
# Edit configuration
sudo nano /etc/ippan/configs/production-node.toml

# Validate configuration
./target/release/ippan-node --config /etc/ippan/configs/production-node.toml --validate

# Reload configuration
sudo systemctl reload ippan
```

#### Configuration Backup
```bash
# Backup current configuration
sudo cp /etc/ippan/configs/production-node.toml /etc/ippan/configs/production-node.toml.backup

# Restore configuration
sudo cp /etc/ippan/configs/production-node.toml.backup /etc/ippan/configs/production-node.toml
sudo systemctl reload ippan
```

## Network Management

### 1. Peer Management

#### List Connected Peers
```bash
# Via API
curl http://localhost:8080/network/peers

# Via CLI
./target/release/ippan-cli network peers

# Via logs
grep "Peer connected" /var/lib/ippan/logs/ippan.log
```

#### Add Bootstrap Peers
```bash
# Add peer via API
curl -X POST http://localhost:8080/network/peers \
  -H "Content-Type: application/json" \
  -d '{"peer_id": "12D3KooW...", "address": "peer.ippan.net:30333"}'

# Add peer via CLI
./target/release/ippan-cli network add-peer 12D3KooW...@peer.ippan.net:30333
```

#### Remove Peers
```bash
# Remove peer via API
curl -X DELETE http://localhost:8080/network/peers/12D3KooW...

# Remove peer via CLI
./target/release/ippan-cli network remove-peer 12D3KooW...
```

### 2. Network Diagnostics

#### Test Connectivity
```bash
# Test P2P connectivity
telnet peer.ippan.net 30333

# Test API connectivity
curl http://peer.ippan.net:8080/health

# Network latency test
ping peer.ippan.net

# Traceroute
traceroute peer.ippan.net
```

#### Network Statistics
```bash
# Network metrics
curl http://localhost:8080/metrics/network

# Connection statistics
netstat -an | grep :30333

# Bandwidth usage
iftop -i eth0
```

### 3. Network Troubleshooting

#### Common Network Issues
```bash
# Check firewall
sudo ufw status
sudo iptables -L

# Check DNS resolution
nslookup peer.ippan.net
dig peer.ippan.net

# Check routing
ip route show
traceroute peer.ippan.net

# Check network interfaces
ip addr show
ip link show
```

## Consensus Operations

### 1. Consensus Status

#### Check Consensus Status
```bash
# Consensus health
curl http://localhost:8080/consensus/status

# Validator status
curl http://localhost:8080/consensus/validators

# Block production status
curl http://localhost:8080/consensus/blocks/latest
```

#### Consensus Metrics
```bash
# Consensus metrics
curl http://localhost:8080/metrics/consensus

# Block time statistics
curl http://localhost:8080/consensus/blocks/stats

# Finality statistics
curl http://localhost:8080/consensus/finality
```

### 2. Validator Operations

#### Join as Validator
```bash
# Create validator account
./target/release/ippan-cli wallet create-validator-account

# Stake tokens
./target/release/ippan-cli staking stake 1000000

# Register validator
./target/release/ippan-cli staking register-validator
```

#### Validator Management
```bash
# Check validator status
./target/release/ippan-cli staking validator-status

# Update commission rate
./target/release/ippan-cli staking update-commission 0.05

# Unstake tokens
./target/release/ippan-cli staking unstake 500000
```

### 3. Consensus Troubleshooting

#### Common Consensus Issues
```bash
# Check consensus logs
grep -i consensus /var/lib/ippan/logs/ippan.log

# Check validator set
curl http://localhost:8080/consensus/validators

# Check block production
curl http://localhost:8080/consensus/blocks/recent

# Check finality
curl http://localhost:8080/consensus/finality
```

## Transaction Management

### 1. Transaction Operations

#### Submit Transaction
```bash
# Submit transaction via API
curl -X POST http://localhost:8080/transactions \
  -H "Content-Type: application/json" \
  -d '{
    "from": "sender_address",
    "to": "recipient_address",
    "amount": 1000,
    "fee": 10,
    "signature": "transaction_signature"
  }'

# Submit transaction via CLI
./target/release/ippan-cli wallet send 1000 recipient_address
```

#### Transaction Status
```bash
# Check transaction status
curl http://localhost:8080/transactions/tx_hash

# Transaction history
curl http://localhost:8080/transactions/history/sender_address

# Pending transactions
curl http://localhost:8080/transactions/pending
```

### 2. Transaction Pool Management

#### Pool Statistics
```bash
# Transaction pool status
curl http://localhost:8080/transactions/pool/status

# Pool size
curl http://localhost:8080/transactions/pool/size

# Pool metrics
curl http://localhost:8080/metrics/transactions
```

#### Pool Operations
```bash
# Clear transaction pool
curl -X POST http://localhost:8080/transactions/pool/clear

# Set pool limits
curl -X POST http://localhost:8080/transactions/pool/limits \
  -H "Content-Type: application/json" \
  -d '{"max_size": 10000, "max_memory": "1GB"}'
```

### 3. Transaction Troubleshooting

#### Common Transaction Issues
```bash
# Check transaction logs
grep -i transaction /var/lib/ippan/logs/ippan.log

# Check transaction pool
curl http://localhost:8080/transactions/pool/status

# Check network connectivity
curl http://localhost:8080/network/peers

# Check consensus status
curl http://localhost:8080/consensus/status
```

## Wallet Operations

### 1. Account Management

#### Create Account
```bash
# Create new account
./target/release/ippan-cli wallet create-account

# Create account with specific type
./target/release/ippan-cli wallet create-account --type validator

# Import account from private key
./target/release/ippan-cli wallet import-account private_key
```

#### Account Operations
```bash
# List accounts
./target/release/ippan-cli wallet list-accounts

# Get account balance
./target/release/ippan-cli wallet balance account_address

# Get account info
./target/release/ippan-cli wallet account-info account_address
```

### 2. Transaction Operations

#### Send Transactions
```bash
# Send simple transaction
./target/release/ippan-cli wallet send 1000 recipient_address

# Send with custom fee
./target/release/ippan-cli wallet send 1000 recipient_address --fee 20

# Send with memo
./target/release/ippan-cli wallet send 1000 recipient_address --memo "Payment for services"
```

#### Transaction History
```bash
# Get transaction history
./target/release/ippan-cli wallet history account_address

# Get transaction details
./target/release/ippan-cli wallet transaction tx_hash

# Export transaction history
./target/release/ippan-cli wallet export-history account_address --format csv
```

### 3. Wallet Security

#### Key Management
```bash
# Backup wallet
./target/release/ippan-cli wallet backup --output wallet_backup.json

# Restore wallet
./target/release/ippan-cli wallet restore --input wallet_backup.json

# Change password
./target/release/ippan-cli wallet change-password
```

#### Security Operations
```bash
# Lock wallet
./target/release/ippan-cli wallet lock

# Unlock wallet
./target/release/ippan-cli wallet unlock

# Check wallet security status
./target/release/ippan-cli wallet security-status
```

## Monitoring and Alerting

### 1. Metrics Collection

#### System Metrics
```bash
# CPU usage
top -p $(pgrep ippan-node)

# Memory usage
free -h
ps aux | grep ippan-node

# Disk usage
df -h /var/lib/ippan
du -sh /var/lib/ippan/data

# Network usage
iftop -i eth0
netstat -i
```

#### Application Metrics
```bash
# Node metrics
curl http://localhost:9090/metrics

# Consensus metrics
curl http://localhost:9090/metrics/consensus

# Network metrics
curl http://localhost:9090/metrics/network

# Storage metrics
curl http://localhost:9090/metrics/storage
```

### 2. Log Monitoring

#### Log Analysis
```bash
# Real-time log monitoring
tail -f /var/lib/ippan/logs/ippan.log

# Error log analysis
grep -i error /var/lib/ippan/logs/ippan.log

# Performance log analysis
grep -i performance /var/lib/ippan/logs/ippan.log

# Security log analysis
grep -i security /var/lib/ippan/logs/ippan.log
```

#### Log Aggregation
```bash
# Send logs to centralized system
rsyslog -f /etc/rsyslog.d/ippan.conf

# Log rotation
logrotate -f /etc/logrotate.d/ippan

# Log compression
gzip /var/lib/ippan/logs/ippan.log.1
```

### 3. Alerting

#### Alert Configuration
```bash
# Check alert status
curl http://localhost:9090/alerts

# Configure alerts
curl -X POST http://localhost:9090/alerts \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High CPU Usage",
    "condition": "cpu_usage > 80",
    "severity": "warning",
    "notification": "email:admin@ippan.net"
  }'
```

#### Alert Testing
```bash
# Test alert
curl -X POST http://localhost:9090/alerts/test

# Check alert history
curl http://localhost:9090/alerts/history

# Acknowledge alert
curl -X POST http://localhost:9090/alerts/acknowledge/alert_id
```

## Performance Optimization

### 1. Performance Monitoring

#### Performance Metrics
```bash
# Performance dashboard
curl http://localhost:8080/performance/dashboard

# Performance benchmarks
curl http://localhost:8080/performance/benchmarks

# Performance bottlenecks
curl http://localhost:8080/performance/bottlenecks
```

#### Performance Analysis
```bash
# CPU profiling
perf record -p $(pgrep ippan-node)
perf report

# Memory profiling
valgrind --tool=massif ./target/release/ippan-node

# Network profiling
tcpdump -i eth0 port 30333
```

### 2. Performance Tuning

#### System Tuning
```bash
# CPU governor
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Memory optimization
echo 1 | sudo tee /proc/sys/vm/drop_caches

# Network optimization
echo 'net.core.rmem_max = 134217728' | sudo tee -a /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

#### Application Tuning
```bash
# Update configuration for performance
sudo nano /etc/ippan/configs/production-node.toml

# Optimize database
./target/release/ippan-cli database optimize

# Clear caches
./target/release/ippan-cli cache clear
```

### 3. Performance Testing

#### Load Testing
```bash
# Run load test
./target/release/ippan-cli test load --transactions 10000 --duration 300

# Stress test
./target/release/ippan-cli test stress --nodes 5 --duration 600

# Performance benchmark
./target/release/ippan-cli test benchmark --operations 100000
```

## Security Operations

### 1. Security Monitoring

#### Security Status
```bash
# Security dashboard
curl http://localhost:8080/security/dashboard

# Security audit
curl http://localhost:8080/security/audit

# Vulnerability scan
curl http://localhost:8080/security/scan
```

#### Security Logs
```bash
# Security event logs
grep -i security /var/lib/ippan/logs/ippan.log

# Authentication logs
grep -i auth /var/lib/ippan/logs/ippan.log

# Access logs
grep -i access /var/lib/ippan/logs/ippan.log
```

### 2. Security Maintenance

#### Certificate Management
```bash
# Check certificate expiration
openssl x509 -in /etc/ippan/certs/server.crt -noout -dates

# Renew certificate
sudo certbot renew --cert-name ippan

# Update certificate in node
sudo systemctl reload ippan
```

#### Access Control
```bash
# Review access logs
grep -i "access denied" /var/lib/ippan/logs/ippan.log

# Update firewall rules
sudo ufw status
sudo ufw allow from 10.0.0.0/8 to any port 30333

# Review user permissions
ls -la /etc/ippan/
```

### 3. Incident Response

#### Security Incident Response
```bash
# Isolate compromised node
sudo systemctl stop ippan
sudo ufw deny from any to any port 30333

# Collect evidence
sudo cp -r /var/lib/ippan/logs /tmp/incident_logs
sudo cp /etc/ippan/configs/production-node.toml /tmp/incident_config

# Notify security team
curl -X POST http://localhost:8080/security/incident \
  -H "Content-Type: application/json" \
  -d '{"severity": "high", "description": "Security incident detected"}'
```

## Backup and Recovery

### 1. Backup Operations

#### Database Backup
```bash
# Create backup
./target/release/ippan-cli database backup --output /var/backups/ippan/db_backup_$(date +%Y%m%d_%H%M%S).sql

# Verify backup
./target/release/ippan-cli database verify-backup /var/backups/ippan/db_backup_20240101_020000.sql

# List backups
ls -la /var/backups/ippan/
```

#### Configuration Backup
```bash
# Backup configuration
sudo tar -czf /var/backups/ippan/config_backup_$(date +%Y%m%d_%H%M%S).tar.gz /etc/ippan/

# Backup wallet
./target/release/ippan-cli wallet backup --output /var/backups/ippan/wallet_backup_$(date +%Y%m%d_%H%M%S).json
```

### 2. Recovery Operations

#### Database Recovery
```bash
# Stop node
sudo systemctl stop ippan

# Restore database
./target/release/ippan-cli database restore --input /var/backups/ippan/db_backup_20240101_020000.sql

# Start node
sudo systemctl start ippan

# Verify recovery
curl http://localhost:8080/health
```

#### Full System Recovery
```bash
# Stop all services
sudo systemctl stop ippan

# Restore from backup
sudo tar -xzf /var/backups/ippan/full_backup_20240101_020000.tar.gz -C /

# Restore configuration
sudo cp /var/backups/ippan/config_backup_20240101_020000.tar.gz /tmp/
sudo tar -xzf /tmp/config_backup_20240101_020000.tar.gz -C /

# Start services
sudo systemctl start ippan
```

### 3. Disaster Recovery

#### Disaster Recovery Plan
```bash
# Activate disaster recovery
./target/release/ippan-cli disaster-recovery activate

# Restore from remote backup
./target/release/ippan-cli disaster-recovery restore --remote-backup s3://ippan-backups/latest/

# Verify disaster recovery
./target/release/ippan-cli disaster-recovery verify
```

## Troubleshooting

### 1. Common Issues

#### Node Won't Start
```bash
# Check logs
journalctl -u ippan -f

# Check configuration
./target/release/ippan-node --config /etc/ippan/configs/production-node.toml --validate

# Check ports
netstat -tlnp | grep :30333

# Check permissions
ls -la /var/lib/ippan/
```

#### Network Issues
```bash
# Check connectivity
ping bootstrap1.ippan.net
telnet bootstrap1.ippan.net 30333

# Check DNS
nslookup bootstrap1.ippan.net
dig bootstrap1.ippan.net

# Check firewall
sudo ufw status
sudo iptables -L
```

#### Performance Issues
```bash
# Check system resources
htop
iostat -x 1
df -h

# Check node metrics
curl http://localhost:9090/metrics

# Check network latency
ping bootstrap1.ippan.net
```

### 2. Diagnostic Tools

#### System Diagnostics
```bash
# System information
uname -a
cat /etc/os-release
lscpu
free -h
df -h

# Network information
ip addr show
ip route show
ss -tlnp
```

#### Application Diagnostics
```bash
# Node information
./target/release/ippan-cli node info

# Network status
./target/release/ippan-cli network status

# Consensus status
./target/release/ippan-cli consensus status

# Storage status
./target/release/ippan-cli storage status
```

### 3. Debug Mode

#### Enable Debug Logging
```bash
# Update configuration
sudo nano /etc/ippan/configs/production-node.toml

# Set log level to debug
[logging]
log_level = "debug"

# Restart node
sudo systemctl restart ippan

# Monitor debug logs
tail -f /var/lib/ippan/logs/ippan.log
```

## Maintenance Procedures

### 1. Regular Maintenance

#### Daily Tasks
```bash
# Check node status
sudo systemctl status ippan

# Check logs for errors
grep -i error /var/lib/ippan/logs/ippan.log

# Check disk space
df -h /var/lib/ippan

# Check network connectivity
curl http://localhost:8080/health
```

#### Weekly Tasks
```bash
# Update system packages
sudo apt update && sudo apt upgrade

# Review security logs
grep -i security /var/lib/ippan/logs/ippan.log

# Check certificate expiration
openssl x509 -in /etc/ippan/certs/server.crt -noout -dates

# Review performance metrics
curl http://localhost:8080/performance/dashboard
```

#### Monthly Tasks
```bash
# Update IPPAN software
cd /opt/ippan
git pull
cargo build --release

# Review and rotate certificates
sudo certbot renew

# Analyze performance trends
curl http://localhost:8080/performance/trends

# Update documentation
git pull origin main
```

### 2. Maintenance Windows

#### Planned Maintenance
```bash
# Schedule maintenance
./target/release/ippan-cli maintenance schedule --start "2024-01-01 02:00:00" --duration 60

# Notify users
curl -X POST http://localhost:8080/notifications \
  -H "Content-Type: application/json" \
  -d '{"message": "Scheduled maintenance in 1 hour", "type": "maintenance"}'

# Execute maintenance
./target/release/ippan-cli maintenance execute
```

#### Emergency Maintenance
```bash
# Emergency stop
sudo systemctl stop ippan

# Emergency procedures
./target/release/ippan-cli emergency procedures

# Emergency recovery
./target/release/ippan-cli emergency recovery
```

### 3. Maintenance Documentation

#### Maintenance Logs
```bash
# Log maintenance activities
echo "$(date): Performed routine maintenance" >> /var/log/ippan-maintenance.log

# Document changes
echo "$(date): Updated configuration" >> /var/log/ippan-changes.log

# Track issues
echo "$(date): Resolved network connectivity issue" >> /var/log/ippan-issues.log
```

## Support and Resources

### 1. Documentation
- **Deployment Guide**: [docs/DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md)
- **API Documentation**: [docs/API_REFERENCE.md](API_REFERENCE.md)
- **Configuration Reference**: [docs/CONFIGURATION.md](CONFIGURATION.md)

### 2. Community Support
- **GitHub Issues**: https://github.com/your-org/ippan/issues
- **Discord Community**: https://discord.gg/ippan
- **Forum**: https://forum.ippan.net

### 3. Professional Support
- **Email Support**: support@ippan.net
- **Enterprise Support**: enterprise@ippan.net
- **Emergency Support**: emergency@ippan.net

## License

This operational guide is part of the IPPAN project and is licensed under the MIT License.
