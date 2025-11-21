# IPPAN RC Threat Model

> Mainnet gating: see `docs/release/v1-mainnet-checklist.md` for launch-blocking security tasks.

## 1. Assets

- Consensus state (blocks, rounds, HashTimers, DLC scores)
- User balances and transactions
- Validator reputation and fairness scores
- Node availability and uptime
- DHT metadata for files and handles

## 2. Trust assumptions

- Honest-majority assumptions for deterministic DLC/FBA-style consensus: a super-majority of validators follow the protocol and do not equivocate over long periods.
- Node operators maintain patched, correctly configured systems and do not bypass runtime safeguards (rate limits, circuit breakers, audit logging).
- End users and client SDKs provide well-formed requests but may be untrusted for authorization; validation and rate limiting still apply.
- The network is not trusted: peers can be byzantine, replay traffic, or attempt DoS.
- External infrastructure (time sources, storage volumes) may be unreliable; the system must degrade safely.

## 3. Adversary model

- Network attacker capable of DoS, spam, replay, or malformed packet injection over RPC and P2P.
- Malicious validator or operator attempting to skew consensus decisions, spam announcements, or bypass fairness tracking.
- Resource-exhaustion attacker flooding RPC endpoints, DHT announcements, or repeated failed authentications to exhaust CPU/memory.
- Key compromise of a node identity: attacker can impersonate the node until keys are rotated; other nodes rely on rate limits, validation, and peer scoring to contain damage.

## 4. Key mitigations in RC

- Rate limiting with per-IP, per-endpoint, and optional global quotas; optional IP whitelisting/blacklisting controls.
- RPC validation for payload shape and error handling to reject malformed bodies; circuit breaker integration to back off after repeated failures.
- P2P message validation for basic shape plus peer metadata tracking; peer caps and discovery throttling to limit churn.
- Deterministic DLC/HashTimer consensus protections to keep block/round ordering stable under network variance.

## 5. Known gaps / deferred items

- External third-party security audit pending (Phase 2).
- No formal proofs yet for DLC economics or incentive compatibility.
- DoS handling is partially covered (application-level rate limits and peer caps) but lacks OS-level firewall/sandboxing guidance in RC.
- ZK-STARK proof system remains in design; not enforced in RC builds.
