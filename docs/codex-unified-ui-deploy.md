# Codex Unified UI Deployment Instructions

Paste the following message into the PR or issue comment so Codex can run the Unified UI deployment workflow:

```
Codex, finish Unified UI deployment

Use workflow: .github/workflows/deploy-unified-ui.yml

Deploy host: ${{ secrets.DEPLOY_HOST }}

SSH user: ${{ secrets.DEPLOY_USER }}

SSH key: ${{ secrets.DEPLOY_SSH_KEY }}

Fingerprint: ${{ secrets.DEPLOY_FINGERPRINT }}

Tasks:

Build & push images:

ghcr.io/${{ github.repository }}/gateway:latest

ghcr.io/${{ github.repository }}/unified-ui:latest

Copy files to server:

deploy/gateway/docker-compose.yml

deploy/gateway/.env

Target: ~/apps/ippan-gateway

On server:

docker compose pull

docker compose up -d

docker compose ps

Confirm full UI enabled with NEXT_PUBLIC_ENABLE_FULL_UI=true.

Post logs from:

docker compose logs --tail=100 unified-ui

docker compose logs --tail=100 gateway

Quick sanity checks (you can run these)

From your PC

ssh -i "$env:USERPROFILE\.ssh\ippan_ci" ubuntu@188.245.97.41 "docker compose -f ~/apps/ippan-gateway/docker-compose.yml ps"
ssh -i "$env:USERPROFILE\.ssh\ippan_ci" ubuntu@188.245.97.41 "docker compose -f ~/apps/ippan-gateway/docker-compose.yml logs --tail=50 unified-ui"

HTTP check

curl -I http://188.245.97.41

Once DNS/TLS is pointed:

curl -I http://188.245.97.41:3001

If the job still fails

“can’t connect without a private SSH key” → in the SSH/SCp steps make sure you have:

key: ${{ secrets.DEPLOY_SSH_KEY }}
fingerprint: ${{ secrets.DEPLOY_FINGERPRINT }}
host: ${{ secrets.DEPLOY_HOST }}
username: ${{ secrets.DEPLOY_USER }}
port: ${{ secrets.DEPLOY_PORT }}

Permission denied → confirm you can SSH manually:

ssh -i "$env:USERPROFILE\.ssh\ippan_ci" ubuntu@188.245.97.41 "echo OK"

Compose not found → install Docker/Compose on server or run:

sudo apt-get update -y && sudo apt-get install -y docker.io docker-compose-plugin
sudo usermod -aG docker $USER

(log out/in once)

That’s it. Run the workflow now; Codex (and CI) will take it from here.
```
