# IPPAN Manual Deployment Guide

## 🚨 IMPORTANT: Manual Deployment Required

The automated deployment failed because the servers are not accessible via SSH with the available credentials. You need to manually deploy using the following steps:

## 📋 Prerequisites

1. **Server Access**: You need to be able to access your servers via:
   - Hetzner Cloud Console (web interface)
   - SSH with the correct credentials
   - Rescue mode access

2. **Files Ready**: The following files are prepared in your project:
   - `server1_deploy_commands.txt` - Commands for Server 1 (188.245.97.41)
   - `server2_deploy_commands.txt` - Commands for Server 2 (135.181.145.174)

## 🔧 Step-by-Step Manual Deployment

### Option 1: Using Hetzner Cloud Console (Recommended)

1. **Go to Hetzner Cloud Console**
   - Visit: https://console.hetzner-cloud.com/
   - Login with your credentials

2. **Access Server 1 (188.245.97.41)**
   - Click on your server
   - Click "Console" tab
   - This will open a web-based terminal

3. **Deploy to Server 1**
   - Open `server1_deploy_commands.txt` on your local machine
   - Copy ALL the commands from that file
   - Paste them into the Hetzner Console terminal
   - Press Enter to execute
   - Wait 5-10 minutes for completion

4. **Access Server 2 (135.181.145.174)**
   - Click on your second server
   - Click "Console" tab
   - This will open a web-based terminal

5. **Deploy to Server 2**
   - Open `server2_deploy_commands.txt` on your local machine
   - Copy ALL the commands from that file
   - Paste them into the Hetzner Console terminal
   - Press Enter to execute
   - Wait 5-10 minutes for completion

### Option 2: Using SSH (if you have access)

1. **Connect to Server 1**
   ```bash
   ssh root@188.245.97.41
   ```

2. **Deploy to Server 1**
   - Copy commands from `server1_deploy_commands.txt`
   - Paste and execute

3. **Connect to Server 2**
   ```bash
   ssh root@135.181.145.174
   ```

4. **Deploy to Server 2**
   - Copy commands from `server2_deploy_commands.txt`
   - Paste and execute

## 🔍 Verification

After deployment, test these URLs:

### Server 1 (188.245.97.41)
- **API Health**: http://188.245.97.41:3000/health
- **Metrics**: http://188.245.97.41:9090
- **P2P Port**: 188.245.97.41:8080

### Server 2 (135.181.145.174)
- **API Health**: http://135.181.145.174:3000/health
- **Metrics**: http://135.181.145.174:9090
- **P2P Port**: 135.181.145.174:8080

## 🎯 Expected Results

After successful deployment:

1. **Docker containers running**:
   ```bash
   docker ps
   ```
   Should show `ippan-node` containers

2. **API responding**:
   - Health endpoint returns 200 OK
   - Metrics endpoint accessible

3. **P2P network**:
   - Both nodes can communicate
   - Blockchain network operational

## 🚨 Troubleshooting

### If deployment fails:

1. **Check server status**:
   ```bash
   systemctl status docker
   docker ps -a
   ```

2. **Check logs**:
   ```bash
   docker logs ippan-node
   tail -f /opt/ippan/mainnet/logs/setup-complete.log
   ```

3. **Restart services**:
   ```bash
   cd /opt/ippan/mainnet
   docker-compose restart
   ```

### If servers are not accessible:

1. **Enable rescue mode**:
   - Use Hetzner Cloud Console
   - Go to server settings
   - Enable rescue mode
   - Get the rescue password

2. **Check firewall**:
   ```bash
   ufw status
   ```

## 📞 Support

If you encounter issues:

1. Check the deployment logs
2. Verify server connectivity
3. Ensure all ports are open (22, 80, 443, 3000, 8080, 9090, 3001)
4. Check Docker and Docker Compose installation

## 🎉 Success!

Once both servers are deployed and APIs are responding, you'll have a fully functional IPPAN blockchain network with:

- ✅ Two blockchain nodes
- ✅ P2P communication
- ✅ REST API endpoints
- ✅ Metrics and monitoring
- ✅ Secure configuration
- ✅ Firewall protection

The blockchain network will be ready for transactions and smart contracts!
