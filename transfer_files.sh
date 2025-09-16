#!/bin/bash
set -e

echo "ðŸ“¦ Transferring REAL MODE blockchain files..."

# Transfer to Server 1
echo "Transferring to Server 1..."
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/ippan-node.exe root@188.245.97.41:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/genesis.json root@188.245.97.41:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/node-a.json root@188.245.97.41:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/ippan.service root@188.245.97.41:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deploy_server1.sh root@188.245.97.41:/tmp/deploy.sh

# Transfer to Server 2
echo "Transferring to Server 2..."
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/ippan-node.exe root@135.181.145.174:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/genesis.json root@135.181.145.174:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/node-b.json root@135.181.145.174:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deployment_temp/ippan.service root@135.181.145.174:/tmp/
scp -o StrictHostKeyChecking=no -o ConnectTimeout=10 deploy_server2.sh root@135.181.145.174:/tmp/deploy.sh

echo "âœ… Files transferred to both servers"
