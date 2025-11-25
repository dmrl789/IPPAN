# DAG-Fair Emission System Integration

The **DAG-Fair Emission System** introduces deterministic, capped, and fair validator reward emission into the IPPAN blockchain.  
It ensures sustainable supply growth, predictable halving, transparent distribution, and full governance control.

---

## 🏗️ Architecture Overview

The current integration enforces the economics rules called out in
`docs/DAG_FAIR_EMISSION.md` and `docs/FEES_AND_EMISSION.md`:

- **Capped supply:** `ippan_economics::EmissionEngine` clamps per-round emission to
  the 21M IPN supply ceiling and the consensus tracker rejects any schedule that
  would overflow the cap.
- **No burn path:** All fees and emission slices are either paid immediately to
  validators or routed into the network dividend pool — there is no residual
  burn or leakage path.
- **Weekly redistribution:** 5% of every round’s emission plus 75% of collected
  fees accumulate in the dividend pool and are redistributed on each audit
  interval (weekly cadence by default) based on observed validator weights.

This keeps per-round rewards deterministic while guaranteeing that fees never
leave the system and dividend payouts are replayable across nodes.

