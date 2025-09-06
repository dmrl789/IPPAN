# AI Ops for IPPAN — PRD + Starter Kit

This package shows **how to create an AI to manage IPPAN** across devnet/testnet/prod. It includes a pragmatic PRD and a runnable starter kit (Rust control-plane + Python AI agent) you can paste into a repo.

---

## 1) Mission & Scope
**Goal:** an autonomous-but-safe AI Operator that keeps IPPAN nodes healthy, secure, and cost‑efficient, and assists humans with diagnostics and routine ops.

**Initial Scope (P0 → P2):**
- **P0 – Observe:** ingest metrics/logs; summarize health; chat-style Q&A over telemetry; create human‑approved runbooks.
- **P1 – Act (guard‑railed):** restart/rotate node, prune storage, rotate keys, update config, rebalance peers; all via allow‑list + dry‑run + approvals.
- **P2 – Autopilot:** predictive restarts, peer management, DHT anti‑spam micro‑cost tuning, HashTimer drift correction; safe rollbacks + canaries.

**Non‑Goals (for now):** protocol changes, on‑chain governance authorship without human review.

---

## 2) Core Capabilities
1) **Monitoring & Summaries**
- Node liveness, peer count, block/round rates, mempool depth, CPU/RAM/IO, disk pressure, network RTT, IPPAN Time drift.
- Daily “ops digest” with anomalies + recommended actions.

2) **Safe Operations**
- Restart node service; rotate/compress logs; set max peers; throttle availability announcements (micro‑costs); update config; prune old state.
- Cross‑platform (Windows service, systemd on Linux, macOS launchd).

3) **Optimization**
- Peer list hygiene (drop flapping peers, prefer low RTT & high uptime).
- Dynamic back‑pressure on gossip/dag relay.

4) **Security Guardrails**
- Allow‑listed commands only; dual‑approval for sensitive changes; dry‑run mode; signed change‑records; secrets vault (no plain‑text keys).

5) **Assistant UX**
- “Explain last 24h finality spikes”, “Generate incident report”, “What changed before mempool surge?”, “Plan blue/green upgrade”.

---

## 3) Architecture
```
+-------------------------+       +-------------------------+       +-------------------+
|  AI Agent (Python)     | <---> | Control Plane (Rust)    | <---- | IPPAN Node/CLI    |
| - Planner + Rules      |       | - REST API (Axum)       |       | - ippan-node      |
| - Policy/Approvals     |       | - Cmd allow-list exec   |       | - ippan-cli       |
| - Summaries/Reports    |       | - Health adapters       |       +-------------------+
+-----------+-------------+       +------------+------------+
            |                                  |
            v                                  v
     Grafana/Prometheus (optional)      Logs (Loki/Vector optional)
```
**Why this split?** The Rust plane is small, fast, and safe to run near the node (least privilege). The Python agent iterates quickly, hosts the AI logic, and calls the Rust plane over HTTP.

---

## 4) Control‑Plane REST (v1)
- `GET /api/v1/health` → `{ peers, block_rate, round_finality_ms, mempool, time_drift_ms, last_block_ts }`
- `POST /api/v1/actions/exec` → run an **allow‑listed** command
  - Body: `{ cmd, args[], timeout_ms?, dry_run? }`
  - Response: `{ accepted, dry_run, stdout, stderr, code }`
- `GET /api/v1/policy` → `{ allowlist: ["ippan-node","ippan-cli","sc","systemctl"], dry_run_default: true }`

> The health endpoint can read `ippan-cli status --json` when available; otherwise it returns best‑effort signals.

---

## 5) Safety & Approvals
- **Allow‑list only**: disallow arbitrary shells.
- **Dry‑run by default**: prints the command, no side effects.
- **2‑phase commit** (optional): `propose` → `approve` → `execute`.
- **Change journal**: signed JSONL with HashTimer/UTC stamps.

---

## 6) Data to Collect (P0)
- Node: peers, inbound/outbound rates, block/round throughput, mempool size, bad/good gossip ratio.
- Host: CPU, RAM, disk usage, IO wait, net RTT.
- Time: IPPAN Time drift vs system clock (aim ≤ 100 ms for IPPAN).

---

## 7) Heuristics (P1 examples)
- **Peer hygiene**: if `peers < 2` for > 60s → try bootstrap list; if still < 2 → restart service.
- **Time drift**: if `|drift| > 100 ms` → resync time; record event.
- **Block stagnation**: if `last_block_age > 2x target` → leak check (logs), then restart.
- **Mempool spike**: if spike + low peers → raise announce micro‑cost temporarily.

---

## 8) Deployment Profiles
- **Dev (solo laptop)**: run Rust plane + Python agent locally.
- **Server (Linux)**: systemd units, optional Prometheus & Grafana.
- **Windows**: NSSM/SC.exe service; paths via env.

---

## 9) Starter Kit — File Map
```
ai-ippan/
  control-plane/           # Rust (Axum)
    Cargo.toml
    src/main.rs
  agent/                   # Python (rules + planner)
    pyproject.toml
    src/agent/__init__.py
    src/agent/config.py
    src/agent/policy.py
    src/agent/health.py
    src/agent/executor.py
    src/agent/brain.py
    src/agent/run.py
  policies/policies.yaml
  deploy/docker-compose.yml (optional)
  prompts/system.md (optional)
```

---

## 10) Starter Code — Rust Control Plane
=== filepath: control-plane/Cargo.toml ===
```toml
[package]
name = "ippan-control"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
thiserror = "1"
which = "6"
```

=== filepath: control-plane/src/main.rs ===
```rust
use axum::{routing::{get, post}, Router, Json};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, process::Stdio, time::Duration};
use tokio::{process::Command, time::timeout};
use tracing::{error, info};

#[derive(Serialize, Default)]
struct Health {
    peers: Option<u32>,
    block_rate: Option<f32>,
    round_finality_ms: Option<u32>,
    mempool: Option<u32>,
    time_drift_ms: Option<i64>,
    last_block_ts: Option<String>,
}

#[derive(Deserialize)]
struct ExecReq {
    cmd: String,
    args: Option<Vec<String>>,
    timeout_ms: Option<u64>,
    dry_run: Option<bool>,
}

#[derive(Serialize)]
struct ExecResp {
    accepted: bool,
    dry_run: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

static ALLOWLIST: &[&str] = &["ippan-node", "ippan-cli", "systemctl", "sc"];

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let app = Router::new()
        .route("/api/v1/health", get(get_health))
        .route("/api/v1/actions/exec", post(post_exec))
        .route("/api/v1/policy", get(get_policy));

    let addr: SocketAddr = "0.0.0.0:8088".parse().unwrap();
    info!("starting control-plane on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_policy() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "allowlist": ALLOWLIST,
        "dry_run_default": true
    }))
}

async fn get_health() -> Json<Health> {
    // Best-effort probe: try `ippan-cli status --json`; otherwise return defaults.
    let mut h = Health::default();
    if which::which("ippan-cli").is_ok() {
        match run("ippan-cli", &["status", "--json"], 2_000).await {
            Ok((code, out, _err)) if code == Some(0) => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&out) {
                    h.peers = val.get("peers").and_then(|v| v.as_u64()).map(|v| v as u32);
                    h.block_rate = val.get("block_rate").and_then(|v| v.as_f64()).map(|v| v as f32);
                    h.round_finality_ms = val.get("round_finality_ms").and_then(|v| v.as_u64()).map(|v| v as u32);
                    h.mempool = val.get("mempool").and_then(|v| v.as_u64()).map(|v| v as u32);
                    h.time_drift_ms = val.get("time_drift_ms").and_then(|v| v.as_i64());
                    h.last_block_ts = val.get("last_block_ts").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }
            Err(e) => error!(?e, "health probe failed"),
            _ => {}
        }
    }
    Json(h)
}

async fn post_exec(Json(req): Json<ExecReq>) -> Json<ExecResp> {
    let dry = req.dry_run.unwrap_or(true);
    let cmd = req.cmd;
    let args = req.args.unwrap_or_default();

    let accepted = ALLOWLIST.iter().any(|a| *a == cmd);
    if !accepted {
        return Json(ExecResp { accepted: false, dry_run: dry, code: None, stdout: String::new(), stderr: format!("command '{}' not allowed", cmd) });
    }

    if dry {
        return Json(ExecResp { accepted: true, dry_run: true, code: None, stdout: format!("DRY-RUN: {} {}", cmd, args.join(" ")), stderr: String::new() });
    }

    match run(&cmd, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), req.timeout_ms.unwrap_or(10_000)).await {
        Ok((code, out, err)) => Json(ExecResp { accepted: true, dry_run: false, code, stdout: out, stderr: err }),
        Err(e) => Json(ExecResp { accepted: true, dry_run: false, code: None, stdout: String::new(), stderr: e.to_string() }),
    }
}

async fn run(cmd: &str, args: &[&str], timeout_ms: u64) -> anyhow::Result<(Option<i32>, String, String)> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let t = timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await?;
    let out = t?;
    Ok((out.status.code(), String::from_utf8_lossy(&out.stdout).to_string(), String::from_utf8_lossy(&out.stderr).to_string()))
}
```

---

## 11) Starter Code — Python AI Agent
=== filepath: agent/pyproject.toml ===
```toml
[project]
name = "ai-ippan-agent"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
  "httpx>=0.27",
  "pydantic>=2.8",
  "typer>=0.12",
]

[project.scripts]
ai-ippan = "agent.run:app"
```

=== filepath: agent/src/agent/__init__.py ===
```python
# empty
```

=== filepath: agent/src/agent/config.py ===
```python
from pydantic import BaseModel
import os

class Config(BaseModel):
    control_url: str = os.getenv("CONTROL_URL", "http://127.0.0.1:8088")
    dry_run_default: bool = True
    peers_min: int = 2
    drift_max_ms: int = 100
    health_interval_s: int = 10

CFG = Config()
```

=== filepath: agent/src/agent/policy.py ===
```python
from typing import List

ALLOW: List[str] = ["ippan-node", "ippan-cli", "systemctl", "sc"]

SENSITIVE: List[str] = ["systemctl", "sc"]  # require human approval in prod
```

=== filepath: agent/src/agent/health.py ===
```python
from pydantic import BaseModel
from typing import Optional

class Health(BaseModel):
    peers: Optional[int] = None
    block_rate: Optional[float] = None
    round_finality_ms: Optional[int] = None
    mempool: Optional[int] = None
    time_drift_ms: Optional[int] = None
    last_block_ts: Optional[str] = None
```

=== filepath: agent/src/agent/executor.py ===
```python
import httpx
from typing import List, Tuple
from .config import CFG

async def exec_cmd(cmd: str, args: List[str], dry_run: bool | None = None) -> Tuple[bool, str]:
    dry = CFG.dry_run_default if dry_run is None else dry_run
    async with httpx.AsyncClient(timeout=15) as cli:
        r = await cli.post(f"{CFG.control_url}/api/v1/actions/exec", json={
            "cmd": cmd,
            "args": args,
            "dry_run": dry,
            "timeout_ms": 10_000
        })
        r.raise_for_status()
        data = r.json()
        ok = data.get("accepted", False) and (data.get("code") in (0, None))
        out = data.get("stdout", "") + ("\n" + data.get("stderr", ""))
        return ok, out
```

=== filepath: agent/src/agent/brain.py ===
```python
import httpx
from .config import CFG
from .health import Health
from .executor import exec_cmd

async def fetch_health() -> Health:
    async with httpx.AsyncClient(timeout=10) as cli:
        r = await cli.get(f"{CFG.control_url}/api/v1/health")
        r.raise_for_status()
        return Health(**r.json())

async def tick_once() -> str:
    h = await fetch_health()
    actions: list[str] = []

    # Peer hygiene
    if (h.peers or 0) < CFG.peers_min:
        ok, out = await exec_cmd("ippan-cli", ["peers", "bootstrap"], dry_run=True)
        actions.append(f"peers<min: bootstrap (ok={ok})")
        # staged restart if still bad on next tick (not here)

    # Time drift correction (illustrative; platform-specific in real ops)
    if (h.time_drift_ms or 0) > CFG.drift_max_ms:
        actions.append("drift>max: advise time resync (manual or NTP service)")

    # Block stagnation heuristic (needs last_block_ts in real node JSON)
    # actions.append("stagnation check pending")

    if not actions:
        return "healthy"
    return "; ".join(actions)
```

=== filepath: agent/src/agent/run.py ===
```python
import asyncio
import typer
from .config import CFG
from .brain import tick_once

app = typer.Typer(add_completion=False)

@app.command()
def loop():
    async def _run():
        while True:
            summary = await tick_once()
            print("AIO: ", summary)
            await asyncio.sleep(CFG.health_interval_s)
    asyncio.run(_run())

if __name__ == "__main__":
    app()
```

---

## 12) Optional — Docker Compose
=== filepath: deploy/docker-compose.yml ===
```yaml
version: "3.8"
services:
  control:
    build: ../control-plane
    ports: ["8088:8088"]
    environment:
      - RUST_LOG=info
    restart: unless-stopped
  agent:
    build: ../agent
    environment:
      - CONTROL_URL=http://control:8088
    depends_on: [control]
    restart: unless-stopped
```

---

## 13) Policies (placeholder)
=== filepath: policies/policies.yaml ===
```yaml
allowlist:
  - ippan-node
  - ippan-cli
  - systemctl
  - sc
require_approval:
  - systemctl
  - sc
```

---

## 14) How to Run (Windows & WSL/Linux)
**Rust control-plane**
```bash
# in ai-ippan/control-plane
cargo run
# serves at http://127.0.0.1:8088
```

**Python agent**
```bash
# in ai-ippan/agent
python -m venv .venv && source .venv/bin/activate  # (Windows: .venv\Scripts\activate)
pip install -e .
ai-ippan loop
```

**Test an action (dry-run)**
```bash
curl -X POST http://127.0.0.1:8088/api/v1/actions/exec \
  -H 'content-type: application/json' \
  -d '{"cmd":"ippan-cli","args":["status","--json"],"dry_run":true}'
```

---

## 15) Next Steps for Your Repo (Cursor/TestSprite prompts)
1. **Create repo & paste files**: “Create project *ai-ippan* with the file map above. Ensure code compiles.”
2. **Wire actual health adapter**: “Update `get_health()` to parse the real `ippan-cli status --json` output.”
3. **Windows service**: “Add SC.exe service: `sc create ippan-control binPath= ... start= auto` and same for agent with NSSM.”
4. **Systemd units**: create `ippan-control.service` and `ai-ippan-agent.service` with restart policies.
5. **Approvals UI**: small web page to approve queued actions before execution.
6. **Prometheus**: expose `/metrics` in Rust and add Grafana dashboard.
7. **Secrets**: integrate OS keyring or HashiCorp Vault; never store raw keys.

---

## 16) IPPAN‑Specific Hooks to Add
- **HashTimer drift check**: compute median drift over 5 mins; alert at > 100 ms.
- **DHT announce cost tuner**: if spam ratio rises → temporarily raise micro‑cost.
- **Peer preferencing**: favor peers with best uptime/RTT; drop flappers.
- **Upgrade playbooks**: blue/green node rollouts; revert on error budget exceeded.

---

## 17) Security Notes
- Keep control‑plane on localhost or a private network segment.
- Use mTLS or a reverse proxy with auth when remote.
- Strict allow‑list; no raw shells; log every action with timestamps & hashes.

---

**You now have a minimal, safe skeleton for an AI that manages IPPAN.** Expand the agent’s `tick_once()` with your real heuristics and connect the control‑plane to the true `ippan-cli` JSON outputs to go from demo → production.

