# Copilot Instructions for IPPAN

## Project
- Monorepo for a Rust-based BlockDAG L1 (“IPPAN”) with HashTimer ordering, libp2p networking, Sled metrics, Warp/HTTP RPC, Docker deploy, and a Next.js “Unified UI”.
- Languages: **Rust (primary)**, **TypeScript/Next.js**, **YAML (GitHub Actions, Docker Compose, Nginx/Envoy)**.

## Coding standards (Rust)
- Edition: 2021+. Prefer stable Rust; avoid `unsafe`.
- Use `anyhow`/`thiserror` for errors. No `.unwrap()` / `.expect()` in library code.
- Keep modules small; public APIs documented with `///`.
- Prefer `blake3`, `ed25519-dalek`, `tokio`, `libp2p 0.53+` with explicit features.
- Write tests in the same crate: `cargo test -p <crate>`. Add at least 1 unit test for new logic.
- Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` before proposing fixes.

## Networking / time
- HashTimer + IPPAN Time are canonical. Never use `SystemTime::now()` for ordering; use the time service abstraction.
- Respect port assignments; avoid binding to 8080 if reserved by gateway.

## Frontend
- Next.js app (Unified UI) uses env flags: 
  `NEXT_PUBLIC_ENABLE_FULL_UI=1`, `NEXT_PUBLIC_GATEWAY_URL=http://188.245.97.41:8081/api`, 
  `NEXT_PUBLIC_API_BASE_URL=http://188.245.97.41:7080`, `NEXT_PUBLIC_WS_URL=ws://188.245.97.41:7080/ws`.

## CI/CD
- Workflows must run on `ubuntu-latest`, avoid privileged steps when possible.
- If a port is busy, free it explicitly in workflows (`lsof -ti:<port> | xargs --no-run-if-empty kill -9`) before `docker compose up`.
- Always add `actions/cache` for cargo/nextjs when build times increase.

## PR guidelines
- Small, focused PRs with a clear title and description.
- Include test updates and docs for behavior changes.
- Commit style: `area: short summary` (e.g., `rpc: fix JSON schema mismatch`).

## What to do when something fails
- For failed GitHub Actions: explain and propose a patch to the YAML. 
- For security/code issues: prefer **CodeQL** fix suggestions (**Copilot Autofix**) and propose a minimal diff.
