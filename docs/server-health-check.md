# Server Health Check Report

Date: 2025-10-08T08:12:03Z

## Summary

- The public unified UI health endpoint at `https://ui.ippan.org/health` is still unreachable from this environment; the outbound proxy blocks the TLS tunnel with HTTP 403 so no live metrics were retrieved.
- Because `/health` could not be queried, peer connectivity (for example the `peer_count` metric) remains unknown for this run.
- Unified UI static assets must exist on disk (default: `apps/unified-ui/dist` or the path in `UNIFIED_UI_DIST_DIR`) for the interface to render; when they are missing the node serves JSON APIs only.

## Command Output

```
curl -I https://ui.ippan.org/health
HTTP/1.1 403 Forbidden
content-length: 9
content-type: text/plain
date: Wed, 08 Oct 2025 07:56:45 GMT
server: envoy
connection: close

curl: (56) CONNECT tunnel failed, response 403
```

## Node Connectivity Notes

- Without access to `/health`, peer status could not be confirmed. When connectivity is restored, query the node health endpoint (`http://<rpc-host>:<rpc-port>/health`) and inspect the `peer_count` field—values greater than zero indicate that peers were discovered successfully.
- If the node reports `peer_count: 0`, ensure that the `BOOTSTRAP_NODES` environment variable supplies the expected peer list before starting the service.
- To automate the peer check once access is available, run `deploy/check-nodes.sh --rpc http://<rpc-host>:<rpc-port>` and review the emitted table; the script exits non-zero when peers are missing.

## Automated UI Asset Repair

Run the helper script to (re)build the Unified UI assets and copy them to the directory the node serves:

```bash
deploy/refresh-ui-assets.sh
```

The script performs `npm ci` and `npm run build` in `apps/unified-ui/`, then copies the resulting `dist` directory to the path defined by `UNIFIED_UI_DIST_DIR` (defaults to `apps/unified-ui/dist`). Re-run the node service afterwards so it can mount the refreshed static files.

## Unified UI Visibility Checklist

- Confirm that the static assets exist on the server (`ls -lah ${UNIFIED_UI_DIST_DIR:-apps/unified-ui/dist}`) and that `index.html` is present.
- After ensuring the assets exist, restart the node or the serving process so the new files are picked up.
- If the UI still renders only a short menu, step through `docs/unified_ui_quick_check.md` to validate container health, proxy routing, and environment variables end-to-end.

## Next Steps

Re-run these checks from an environment with direct access to the production servers (for example, via the documented GitHub workflow) to retrieve live health and peer connectivity data.
