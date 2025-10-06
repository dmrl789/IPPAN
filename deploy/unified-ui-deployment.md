# Unified UI Deployment via GitHub Actions

This guide explains how to prepare a target host and configure GitHub Actions so that the `Build & Deploy Unified UI` workflow can publish the latest image to your server.

## 1. Configure repository secrets
Add the following **Actions secrets** in GitHub (`Settings → Secrets and variables → Actions`). These values are required by the `appleboy/ssh-action` step in the deploy job.

| Secret | Description |
| --- | --- |
| `SERVER_HOST` | Public IP address or DNS name of the target server. |
| `SERVER_USER` | SSH user that owns the deployment (for example `root` or `deploy`). |
| `SERVER_SSH_KEY` | Private SSH key for the user above (PEM format with line breaks). |

> Ensure the selected user is allowed to authenticate with keys. If you use `root`, enable `PermitRootLogin yes` and `PubkeyAuthentication yes` in `/etc/ssh/sshd_config`, and confirm that your firewall allows inbound SSH from GitHub runners.

## 2. Bootstrap the server (run once)
Execute the following commands on the server before the first deployment. They install Docker, create the deployment directory, and provide a minimal Compose file and UI environment.

```bash
curl -fsSL https://get.docker.com | sh
usermod -aG docker $USER || true

sudo mkdir -p /srv/ippan
sudo tee /srv/ippan/docker-compose.yml >/dev/null <<'YAML'
services:
  ippan-ui:
    image: ghcr.io/dmrl789/ippan-unified-ui:latest
    env_file: /srv/ippan/ui.env
    ports:
      - "${UI_HOST_PORT:-3001}:3000"
    restart: unless-stopped
YAML

sudo tee /srv/ippan/ui.env >/dev/null <<'ENV'
NODE_ENV=production
PORT=3000
NEXT_PUBLIC_GATEWAY_URL=https://ui.ippan.org/api
NEXT_PUBLIC_API_BASE_URL=https://ui.ippan.org/api
NEXT_PUBLIC_WS_URL=wss://ui.ippan.org/ws
NEXT_PUBLIC_ENABLE_FULL_UI=1
NEXT_PUBLIC_NETWORK_NAME=IPPAN-Devnet
ENV

sudo tee /srv/ippan/.env >/dev/null <<'ENV'
UI_HOST_PORT=3001
ENV
```

If you plan to proxy the UI behind Envoy or Nginx, adjust the exposed port mapping accordingly. The example above binds the
container's port `3000` to host port `3001` so GitHub-hosted runners avoid conflicts with existing services on `3000`.
Docker Compose automatically reads the `.env` file in `/srv/ippan`, so the `UI_HOST_PORT` value applies whenever you run
`docker compose` in that directory.

## 3. Optional: Reverse proxy configuration
Below is an example Nginx server block that proxies incoming traffic to the UI container. Update `server_name` and any TLS settings to match your environment.

```nginx
# Redirect HTTP to HTTPS (optional but recommended)
server {
  listen 80;
  server_name ui.ippan.org;
  return 301 https://$host$request_uri;
}

server {
  listen 443 ssl http2;
  server_name ui.ippan.org;

  ssl_certificate     /etc/letsencrypt/live/ui.ippan.org/fullchain.pem;
  ssl_certificate_key /etc/letsencrypt/live/ui.ippan.org/privkey.pem;

  # Serve the compiled UI
  location / {
    proxy_pass http://127.0.0.1:3001;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_buffering off;
  }

  # Forward all REST requests to the gateway
  location ^~ /api/ {
    proxy_pass http://127.0.0.1:8080/;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_buffering off;
  }

  # Enable WebSocket upgrades for subscription traffic
  location /ws {
    proxy_pass http://127.0.0.1:8080/ws;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_read_timeout 600s;
    proxy_send_timeout 600s;
  }

  location = /health {
    access_log off;
    return 200 "ok\n";
    add_header Content-Type text/plain;
  }
}
```

With this configuration the unified UI is served directly from
`https://ui.ippan.org`, replacing the former
`https://ui.ippan.org/dashboard` address.

For Envoy-based setups, ensure the virtual host configuration includes the incoming domain or use `"*"` to accept all hosts.
The repository now ships with a ready-to-use configuration at
`deployments/envoy/envoy.yaml` that binds Envoy to port `80`, proxies traffic to
the UI container listening on `3000`, and permits any `Host` header. Deploy it
alongside the UI container to avoid the `Domain forbidden` responses that occur
when the requested host is missing from the Envoy allow list:

```bash
docker run --rm -v "$PWD/deployments/envoy/envoy.yaml:/etc/envoy/envoy.yaml" \
  --network host envoyproxy/envoy:v1.31-latest
```

If you need to restrict accepted domains later, replace the wildcard entry in
`domains:` with an explicit list of hostnames while keeping the UI upstream
settings intact.

## 4. Verify deployments
1. Push a change under `apps/unified-ui/**`. The workflow builds and pushes a new image to GHCR, then deploys it to the server.
2. On the server, verify the container is healthy:
   ```bash
   docker ps
   curl -I http://127.0.0.1:3001/api/health
   curl -I http://127.0.0.1:3001/ws --http1.1 -H 'Upgrade: websocket' -H 'Connection: Upgrade'
   ```
3. If proxied, check the public endpoint:
   ```bash
   curl -I https://<server-host>/api/health
   curl -I -H 'Upgrade: websocket' -H 'Connection: Upgrade' https://<server-host>/ws
   ```

Common issues are usually related to SSH authentication, missing Docker/Compose packages, or GHCR authentication. Address those first if the deploy job fails.
