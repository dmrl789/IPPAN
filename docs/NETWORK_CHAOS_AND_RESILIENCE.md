# Network Chaos & Resilience Harness

The IPPAN node now exposes lightweight, opt-in "chaos" controls so developers can rehearse
network partitions, jitter, and packet loss without external tooling. These knobs exist solely
for testing/localnet environments and **must stay at zero in production configs**.

## Chaos configuration knobs

Set these values through the existing `IPPAN_*` environment variables (or inline inside
`localnet/*.toml`). All values default to `0`, which disables chaos completely.

| Variable | Description |
| --- | --- |
| `IPPAN_CHAOS_DROP_OUTBOUND_PROB` | Probability to drop outbound HTTP P2P messages (0–10000 ⇒ 0–100.00%). |
| `IPPAN_CHAOS_DROP_INBOUND_PROB` | Probability to drop inbound HTTP P2P messages. |
| `IPPAN_CHAOS_EXTRA_LATENCY_MS_MIN` | Minimum artificial delay (milliseconds) injected before delivering a message. |
| `IPPAN_CHAOS_EXTRA_LATENCY_MS_MAX` | Maximum artificial delay; automatically clamped ≥ min. |

The HTTP P2P layer samples once per message using a deterministic RNG derived from the node's
listen address. When drop probability is met, the message is silently discarded; otherwise the
handler sleeps for a sampled latency between the configured min/max bounds before continuing.

## Chaos localnet quickstart

1. **Start the three-node localnet with chaos profiles**

   ```bash
   scripts/localnet_chaos_start.sh
   ```

   * Node 1 runs with the default (no chaos) profile.
   * Node 2 injects a light amount of loss + latency.
   * Node 3 injects a heavier amount (higher drop and jitter).

   Override any node's profile by exporting `NODE{1,2,3}_*` env vars before running the script
   (see the script header for the full list).

2. **Exercise the network while chaos is active**

   ```bash
   scripts/localnet_chaos_scenario.sh
   ```

   The scenario script:

   * Generates two demo accounts (or reuses cached keys).
   * Funds both accounts through every node's `/dev/fund` endpoint.
   * Registers a handle on node 1.
   * Sends a payment on node 1 from account A → B.
   * Publishes a file descriptor through node 2.
   * Polls node 3's RPC APIs (payments, handles, files) up to 30 attempts with sleeps so the
     eventual consistency can win despite message drops.

   The script exits non-zero if any artifact fails to propagate to node 3.

## Node churn & restart scenario

`scripts/localnet_churn_scenario.sh` guides you through a manual failure/recovery test:

1. Verifies all three nodes are responding, then funds demo accounts.
2. Stops node 2 (best-effort via `localnet/node2.pid`, then prompts if manual intervention is
   needed) and confirms its `/health` endpoint is offline.
3. While node 2 is down:
   * Sends two cross-node payments (via node 1 and node 3).
   * Registers a new handle and publishes a file via node 3.
4. Prompts you to restart node 2 (e.g., rerun `scripts/localnet_chaos_start.sh`) using the same
   data directory, then waits for `/health` to recover.
5. Polls node 2 for the payments, handle, and file until they become visible, proving the node
   resynchronizes state after a crash/restart.

The script emits rich logging plus the raw JSON returned by RPC so you can inspect propagation
under adverse conditions. It exits with a non-zero status if any piece of data fails to converge.

## Success criteria

* **Consensus propagation:** Payments submitted on one node eventually appear in another node's
  `/account/:address/payments` list even when drops/latency are injected.
* **Handle/DHT propagation:** Newly registered handles replicate across the HTTP P2P network and
  stay queryable via `/handle/{handle}`.
* **File descriptor availability:** Files published on a chaos-affected node remain discoverable on
  healthier peers via `/files/{id}`.
* **Node churn recovery:** After shutting down and restarting a node with the same storage, RPC
  queries reflect all previously submitted operations.

These flows provide a practical smoke test for IPPAN's resilience while still being quick to run
on a laptop. Future work can layer automated, long-running chaos suites (tracked in the
checklist as TODOs).
