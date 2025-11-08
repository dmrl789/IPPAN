# 🤖 IPPAN Agent Charter
**Version:** 2025-11-08  
**Purpose:** Define clear boundaries and collaboration rules for multi-agent development in the IPPAN ecosystem (including IPPAN, FinDAG, and related projects).

---

## 🧩 1. General Principles
You are an **independent autonomous agent** working on a **specific subsystem** of the IPPAN monorepo.  
You must **not** modify or influence code outside your assigned path.

IPPAN is a deterministic blockchain framework — every change must be predictable, reproducible, and testable.

---

## 🗂️ 2. Allowed Scopes

Each agent must restrict edits to one of the following domains:

| Domain | Path | Description |
|---------|------|-------------|
| **AI Core** | `/crates/ai_core` | Deterministic AI logic, D-GBDT, Proof-of-Inference |
| **Consensus** | `/crates/consensus` | DLC consensus, round scheduler, validator scoring |
| **Storage** | `/crates/storage` | Sled and in-memory database abstraction layer |
| **Crypto** | `/crates/crypto` | Keypairs, signing, encryption, address formats |
| **Network / P2P** | `/crates/p2p` | libp2p stack, DHT, NAT traversal |
| **Mempool** | `/crates/mempool` | Transaction validation and relay logic |
| **Types** | `/crates/types` | Shared data structures and serialization |
| **Economics** | `/crates/ippan_economics` | Emission, validator rewards, treasury logic |
| **Gateway (Backend API)** | `/apps/gateway` | Warp/Actix backend exposing JSON and WebSocket APIs |
| **UI (Frontend)** | `/apps/ui` | Next.js user interface and explorer |
| **Docs / Infra** | `/docs`, `.github/`, `/scripts` | Documentation, workflows, build automation |

Work **only inside your assigned path**.  
Read other crates for context if necessary, but never edit them.

---

## 🌱 3. Branching and Commits

- Use a **dedicated branch**:  
  `cursor/<short-description>`  
  e.g. `cursor/fix-ai-determinism`, `cursor/update-gateway-api`.

- Commit format:
```

[agent-name] short summary

````
Example:  
`[ai-core] Replace floating-point math with deterministic fixed-point ops`

- Never rebase, rename, or force-push shared branches.

---

## 🧠 4. Coding Standards

1. Always provide **full file replacements** when refactoring (not partial diffs).  
2. Maintain deterministic, architecture-independent behavior — avoid floating-point, random seeds, and nondeterministic data ordering.  
3. Use consistent imports (`crate::`, `super::`), no circular references.  
4. Include doc comments (`///`) for every new struct, enum, or public function.  
5. Keep functions small, pure, and testable.  
6. Follow Rust 2024 edition idioms and Clippy recommendations (`cargo clippy -- -D warnings`).

---

## 🧪 5. Testing

All code must compile and pass:

```bash
cargo check --workspace
cargo test --workspace --all-features
````

Add or update tests relevant to your scope under `/tests` or local `mod tests`.

---

## ⚙️ 6. CI/CD & Dependencies

* Do **not** modify:

  * Root `Cargo.toml`
  * `Cargo.lock`
  * `.github/workflows`
  * Dependencies or versions

* Only the **infrastructure agent** may edit these files when instructed.

---

## 📜 7. Documentation

If you add public APIs or new components:

* Update the local `README.md` in your crate or `/docs` directory.
* Keep documentation minimal, clear, and factual.

---

## 🧍 8. Collaboration Rules

* Do **not** assume other agents’ unfinished work exists.
* Treat missing code as TODOs or interfaces to implement later.
* Never rename, delete, or move other agents’ modules.
* Avoid touching shared registry files (`lib.rs`, `mod.rs`) unless within your crate.

---

## 🧰 9. Determinism Principles

IPPAN enforces **bit-for-bit reproducibility** across CPU architectures.
Agents must:

* Use integer-based fixed-point arithmetic where applicable.
* Avoid OS- or compiler-specific behavior.
* Ensure consistent serialization order (e.g., `BTreeMap`, sorted vectors).
* Guarantee deterministic random sources (`seed_from_hash(...)` patterns).

---

## 🪄 10. Example Invocation (for new agents)

When you start a new agent in Cursor or Codex, use a message like:

> You are `ai-core-determinism`.
> Follow the Charter defined in `.cursor/AGENT_CHARTER.md`.
> Your scope is `/crates/ai_core`.
> Task: replace floating-point math with deterministic fixed-point operations.
> All code must compile, pass tests, and stay within the assigned path.

---

## ✅ Summary Checklist

* [ ] Stay within assigned folder
* [ ] Full file edits only
* [ ] Deterministic, testable code
* [ ] Add/Update tests
* [ ] No dependency or workflow edits
* [ ] Clean commit message
* [ ] Document all public interfaces

---

### End of Charter

