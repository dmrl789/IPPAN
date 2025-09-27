# IPPAN Unified UI Quick Verification Checklist

Use this checklist to confirm that the IPPAN Unified UI container and its dependencies are deployed correctly. Each step should pass before moving to the next one.

## 1. Confirm the UI container is running
```bash
docker compose -f /srv/ippan/docker-compose.yml ps
```
Expected output includes a line similar to:
```
ippan-ui   Up      0.0.0.0:3000->3000/tcp
```

## 2. Confirm the UI health endpoint locally
```bash
curl -fsS http://127.0.0.1:3000/api/health && echo "UI OK (local)"
```
Expected output:
```
ok
UI OK (local)
```

## 3. Confirm the reverse proxy routes correctly
Replace `<SERVER_IP>` with the server's public IP or domain.
```bash
curl -i http://<SERVER_IP>/api/health | sed -n '1,5p'
```
Expected headers include:
```
HTTP/1.1 200 OK
```

If you see `403 Domain forbidden`, update the virtual host configuration (Envoy/Nginx) so the Host header matches your domain or IP.

## 4. Confirm the UI can reach its RPC backend
Replace the host and port with the values configured in `NEXT_PUBLIC_API_BASE_URL`.
```bash
curl -i http://<RPC_HOST:RPC_PORT>/health || true
curl -i http://<RPC_HOST:RPC_PORT>/version || true
```
Expect HTTP 200 responses or valid JSON payloads.

## 5. Optional: One-shot diagnostic script
Create and run the following helper script to automate the checks above:
```bash
cat <<'SH' > /tmp/ippan_ui_diag.sh
set -e
echo "== Docker Compose ps =="
docker compose -f /srv/ippan/docker-compose.yml ps || true

echo "== Local UI health =="
if curl -fsS http://127.0.0.1:3000/api/health >/dev/null; then
  echo "UI OK (local)"
else
  echo "UI NOT responding locally"; exit 1
fi

echo "== Proxy health (port 80) =="
IP=$(hostname -I | awk '{print $1}')
HTTP=$(curl -s -o /dev/null -w "%{http_code}" http://$IP/api/health || true)
echo "Proxy returned HTTP $HTTP at http://$IP/api/health"
if [ "$HTTP" != "200" ]; then
  echo "Likely virtual host mismatch (Host header) or proxy not routing."
fi

echo "== Env vars fed to container =="
docker inspect $(docker ps -q --filter name=ippan-ui) --format '{{json .Config.Env}}' | jq -r '.[]' 2>/dev/null || true

echo "== Recent UI logs =="
docker logs --tail 100 $(docker ps -q --filter name=ippan-ui) || true
SH
bash /tmp/ippan_ui_diag.sh
```

## Common fixes if a check fails
- **403 from proxy**: Update the proxy virtual host domains and reload.
- **UI blank or erroring**: Ensure the container has the following environment variables:
  ```
  NEXT_PUBLIC_API_BASE_URL=http://<rpc-ip>:<port>
  NEXT_PUBLIC_WS_URL=ws://<rpc-ip>:<port>/ws
  NODE_ENV=production
  PORT=3000
  ```
  Recreate the container with `docker compose up -d`.
- **CORS failures**: Allow the UI origin on the RPC or proxy. For Envoy:
  ```yaml
  cors:
    allow_origin_string_match: [{ safe_regex: { regex: ".*" } }]
    allow_methods: "GET,POST,OPTIONS"
    allow_headers: "content-type,authorization"
    allow_credentials: true
  ```
- **CI deploy missing**: Check GitHub secrets `SERVER_HOST`, `SERVER_USER`, and `SERVER_SSH_KEY`. Deployment runs on pushes to `main` under `apps/unified-ui/**`.
