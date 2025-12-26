## CURSOR IDE ORDER — “3 days no improvement / going backwards” root-cause + fix plan (DevNet api1..api4 + Falkenstein bot)

### GOAL

Stop regressions by proving (with **hard invariants**) whether the problem is:

1. **Ops/config drift** (most likely),
2. **Key/identity duplication**,
3. **Consensus liveness failing while RPC looks “ok”**,
4. **Bot overload / circuit breaker**,
—not “IPPAN is badly designed”.

### NON-NEGOTIABLES

- **Run from WSL bash** for loops/scripts (avoid PowerShell quoting issues).
- Do **not** change consensus rules.
- Any “healthy” node must also be **consensus-live** (round advancing).

---

### Phase runner (recommended)

This repo includes a runnable script that performs PHASE 0–7:

```bash
cd /mnt/c/Users/ugosa/Desktop/Cursor\ SOFTWARE/IPPAN
chmod +x scripts/ops/devnet_rootcause_and_fix.sh scripts/ops/devnet_backwards_fix.sh

export NODES="api1.ippan.uk api2.ippan.uk api3.ippan.uk api4.ippan.uk"
export ROLL="api2.ippan.uk api3.ippan.uk api4.ippan.uk api1.ippan.uk"
export VIDS="df312d197f089118a7095cd466cdf84527f6b4062774b825a41b2371bc874743,460c56d288d1d77a8f0f0a0e6a403fe8c2f0a2fc20e153a34d5ebafc08c520d2,60a53cdfed305dd03e389642e737c5737603b01e6ab0ae0cba8fa46f701860dd,254451713534628ea230235ed2b49dd66e30ae378c631e4e04c07b7a14c2bfcb"
export DLC_PATH="/opt/ippan/config/dlc.toml"

./scripts/ops/devnet_rootcause_and_fix.sh all
```

If you want *only* the pre-fix proofs:

```bash
./scripts/ops/devnet_rootcause_and_fix.sh snapshot
./scripts/ops/devnet_rootcause_and_fix.sh identity
./scripts/ops/devnet_rootcause_and_fix.sh round
./scripts/ops/devnet_rootcause_and_fix.sh systemd api1.ippan.uk
```

If you want *only* the drift fix (PHASE 4/5):

```bash
./scripts/ops/devnet_rootcause_and_fix.sh fix
```

Note: the drift fix now also disables `99-validator-set.conf` (renamed to `.disabled`) so there is **exactly one** validator-set source of truth: `60-devnet-consensus.conf`.

---

### Drift sources (what we explicitly neutralize)

- **Single source of truth**: only `60-devnet-consensus.conf` should set:
  - `IPPAN_VALIDATOR_IDS`
  - `ENABLE_DLC`
  - `IPPAN_DLC_CONFIG_PATH`
- **Must be disabled** (renamed to `.disabled`):
  - `99-consensus-validators.conf` (legacy env key history / drift risk)
  - `99-validator-set.conf` (duplicate validator set source)

Quick verification (example on api1):

```bash
ssh root@api1.ippan.uk 'ls -1 /etc/systemd/system/ippan-node.service.d | egrep "validator-set|consensus-validators"'
```

Expected: both show as `*.disabled` (or do not exist).

And the invariant gate:

```bash
./scripts/ops/devnet_rootcause_and_fix.sh invariants
```

---

### Bot sanity (Falkenstein) — optional but recommended during stabilization

```bash
export BOT_HOST="root@<FALKENSTEIN_IP>"
export BOT_SERVICE="ippan-tx-bot"
./scripts/ops/devnet_rootcause_and_fix.sh bot-pause
```

---

### Conclusion to embed in Cursor notes

If after PHASE 4–7 the network stabilizes (**4 validators**, **rounds advance**, **invariants pass**), then IPPAN design is **not** the reason you were “going backwards”.
The reason is **deployment determinism drift**: multiple systemd drop-ins + mismatched env keys + silent fallbacks.


