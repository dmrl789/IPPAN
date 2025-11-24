# Gateway & Explorer Runbook

The IPPAN gateway/explorer tier proxies public traffic to one or more validator RPC nodes. Harden it like any other Internet-facing API edge.

## 1. Reference architecture

```
Internet ─▶ TLS reverse proxy (nginx/Caddy) ─▶ ippan-gateway (RPC) ─▶ validator RPC (loopback)
                                                        └────▶ explorer UI (static assets)
```

- Place the reverse proxy inside a DMZ/VPC subnet. Only the proxy listens on `0.0.0.0:443`.
- Run the gateway/explorer binaries on the same host or a private VLAN; they should bind to loopback/privates addresses only.

## 2. Configuration

### Environment

```
IPPN_RPC_HOST=127.0.0.1
IPPN_RPC_PORT=8080
IPPN_RPC_ALLOWED_ORIGINS=https://explorer.example.com,https://status.example.com
IPPN_PROMETHEUS_ENABLED=true
```

- Keep `RPC_ALLOWED_ORIGINS` explicit; wildcards (`*`) are **ignored** unless you set `dev_mode=true`.
- Configure the explorer UI (`apps/explorer`) to point at the gateway ingress URL via `NEXT_PUBLIC_IPPAN_RPC_BASE`.

### Reverse proxy (nginx example)

```
server {
    listen 443 ssl http2;
    server_name explorer.example.com;

    ssl_certificate     /etc/letsencrypt/live/explorer/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/explorer/privkey.pem;

    location /api/ {
        proxy_pass http://127.0.0.1:8080/;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        limit_req zone=api burst=100 nodelay;
    }

    location / {
        root /opt/ippan/ui;
        try_files $uri /index.html;
    }
}
```

- Terminate TLS with Let’s Encrypt or another ACM/HSM-backed certificate.
- Apply rate limiting (`limit_req`) and basic WAF rules before traffic reaches the gateway.

## 3. Operations checklist

- **Deployments**: Ship gateway + UI via containers with blue/green rotation. Run smoke tests (`curl /api/health`) before cutting DNS.
- **Monitoring**: Scrape `/metrics` from the gateway host (loopback). Alert on `node_health == 0`, 5xx spikes, or saturation of request limits.
- **Logging**: Forward nginx access logs and gateway structured logs to a central SIEM. Correlate request IDs from reverse proxy → gateway.
- **Backups**: Static UI can be rebuilt; configuration (`/etc/ippan`, TLS certs) should be stored in your secrets manager or IaC repo.

## 4. Security controls

- Enforce mutual TLS or IP allow-lists between gateway and validator RPC nodes.
- Keep the explorer UI read-only; never expose validator keys or wallet secrets at this tier.
- Use CSP headers (`default-src 'self'`) to reduce XSS risk on the explorer domain.
- Rotate TLS certificates automatically (certbot timers or ACM).

## 5. Incident response

- **CORS misconfiguration**: If `RPC_ALLOWED_ORIGINS` accidentally includes `*`, update the env file and restart; the node will reject the wildcard when `dev_mode=false`.
- **Traffic floods**: Tighten rate limits at the reverse proxy, enable geo/IP filtering, and coordinate with upstream scrubbing providers.
- **Compromised UI build**: Rebuild from trusted source, invalidate CDN caches, and rotate CSP/reporting endpoints.

Document every change to gateway/explorer infrastructure in your runbook and automate tests to verify CORS, TLS, and rate limits before promoting new releases.
