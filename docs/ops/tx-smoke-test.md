## Transaction smoke test (safe + bounded)

This runbook validates that your IPPAN node can **accept and confirm real transactions** in a **non-abusive** way.

### Safety rules

- **Only** submit transactions to nodes you own/operate (or have explicit permission to test).
- Keep it small: **start with 1 transaction**, then a tiny batch (e.g., 5–20) with manual pacing.
- If you see elevated errors, rising latency, or services impacted, **stop** and investigate instead of “pushing harder”.

---

## 0) Prereqs

- **RPC URL**: default localnet is `http://127.0.0.1:8080` (see `docs/LOCALNET_QUICKSTART.md`).
- **Wallet CLI**: use the official `ippan-wallet` binary (recommended for “real tx” submission).

This repo’s user guide shows the wallet flow here: `docs/users/getting-started.md`.

---

## 1) Create a wallet key (one-time)

Use the wallet CLI to create a key and display your address.

If you already have a key, skip to step 2.

---

## 2) Fund the wallet (devnet/localnet)

You need funds before you can send a payment.

- Local devnet typically supports a “dev funds” helper in the wallet CLI.
- Alternatively, use whatever faucet mechanism your devnet operator provided.

Verify balance via RPC:

- `GET /account/<address>` (example in `docs/users/getting-started.md`)

---

## 3) Send a single payment (the actual smoke test)

Send **one** payment transaction using the wallet CLI’s `send-payment` command against your RPC URL.

What to capture from the output:

- **Transaction hash**
- Any error message

---

## 4) Verify it was accepted and confirmed

Use the transaction hash from step 3.

- **Transaction status**: `GET /tx/<hash>`
- **Balances** (optional): `GET /account/<from>` and `GET /account/<to>`
- **Node health** (optional): `GET /health` and/or `GET /status`

Expected results:

- `/tx/<hash>` should return the transaction record (and eventually show confirmed/in-block details if your node exposes them).
- Sender balance decreases by (amount + fee), receiver balance increases by amount.

---

## 5) Bounded “small batch” (optional)

If the single transaction works, repeat with a **small batch**:

- 5 to 20 transactions total
- pace them manually (don’t fire them all at once)
- stop on the first sign of instability (errors/timeouts)

Record:

- Success count / failure count
- Approximate end-to-end time
- Any error types (HTTP status, RPC error strings)

---

## 6) If it fails: quick triage checklist

- **Wrong RPC URL / port**: localnet node RPC is usually `:8080`
- **Insufficient funds**: faucet/dev funds step didn’t work
- **Clock/time issues**: check `/time` monotonicity (`docs/ops/time.md`)
- **Mempool/consensus stuck**: check `/status`, peers, and validator count


