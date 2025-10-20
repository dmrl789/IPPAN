# Server 1 Deployment Instructions

## Manual Deployment Steps:

1. Access Server 1 (188.245.97.41) via any available method:
   - SSH (if port 22 becomes accessible)
   - Web interface (if available)
   - Console access (if physical access)
   - Remote desktop (if RDP is enabled)

2. Copy deployment files to Server 1:
   - Upload server1-deployment/ folder to /opt/ippan/
   - Or use: scp -r server1-deployment/ root@188.245.97.41:/opt/ippan/

3. Run deployment:
   `ash
   cd /opt/ippan
   chmod +x deploy-server1.sh
   ./deploy-server1.sh
   `

4. Verify deployment:
   `ash
   docker ps
   curl http://localhost:8080/health
   `

5. Configure firewall:
   `ash
   ufw allow 8080/tcp
   ufw allow 9000/tcp
   `

## Alternative: Use Different Server

If Server 1 cannot be accessed, deploy to a different server:
- Use the same deployment files
- Update IP addresses in configuration
- Update Server 2 bootstrap configuration

## Verification

After deployment, run:
`powershell
.\test-node-connectivity.ps1
`

Expected result:
- Both servers accessible
- Peer count > 0
- Nodes connected
