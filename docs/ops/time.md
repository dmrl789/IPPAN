---
# IPPAN Time — Monotonic TimeState

## Why this matters
IPPAN time must be monotonic (never move backwards). A prior implementation split global time state across multiple mutexes with inconsistent lock ordering, which could produce racey snapshots under parallel execution and carried a real deadlock risk.

## What changed
- Consolidated time state into a single mutex (TimeState).
- now_us() enforces monotonicity unconditionally (peer offset ingestion cannot decrease computed time).
- Intended “median offset” behavior is preserved, but applied without allowing backwards jumps.

## Canary verification (on-node)
1) Service health:
   sudo systemctl status ippan-node --no-pager
   # Note: in the default devnet config, RPC listens on :8080 (P2P is :9000).
   curl -fsS http://127.0.0.1:8080/status | jq .

2) Peers:
   curl -fsS http://127.0.0.1:8080/peers | jq .

3) Monotonic time check:
   for i in $(seq 1 20); do
     curl -fsS http://127.0.0.1:8080/time | jq -r '.time_us'
     sleep 0.2
   done

   Expectation: values should never decrease.

## Rollback (if needed)
- Stop service: sudo systemctl stop ippan-node
- Restore previous binary: sudo cp -a /usr/local/bin/ippan-node.bak.<TIMESTAMP> /usr/local/bin/ippan-node
- Start service: sudo systemctl start ippan-node
- Re-run the health + /time checks.

## Health contract (stable checks for ops/CI)

### /status (RPC)
Fields relied on by ops tooling:
- `status`: string, expected `"ok"`
- `peer_count`: integer (devnet expected 4)
- `version`: string (semantic version)
- `build_sha`: string git commit (for drift detection; may be `"unknown"` if build metadata not embedded)

### /peers (RPC)
- JSON array of peer addresses; devnet expected `length == 4`

### /time (RPC)
- `time_us`: integer microseconds
- Must be monotonic over a short sample window (no decreases across successive samples)
---


