# DLC Long-Run Soak Testing

## Overview

The soak workflow (`soak-dlc-longrun.yml`) stress-tests DLC consensus determinism and stability by repeatedly running the `fairness_invariants` test suite over extended periods. This helps catch:

- Memory leaks
- Non-deterministic behavior
- Resource exhaustion
- Flaky test failures
- Long-term stability issues

## Running the Soak Workflow

### Manual Run (GitHub Actions)

1. Navigate to **Actions** → **Soak — DLC Long-Run Determinism**
2. Click **Run workflow**
3. Configure:
   - **minutes**: Duration in minutes (default: 180)
   - **profile**: Test intensity
     - `smoke`: 20 iterations (quick validation)
     - `standard`: 80 iterations (default)
     - `heavy`: 200 iterations (thorough stress test)
4. Click **Run workflow**

### Scheduled Runs

The workflow runs automatically every **Sunday at 03:00 UTC** with default settings (180 minutes, standard profile).

## Running Locally

To reproduce the soak test locally:

```bash
# Set duration (minutes) and profile
MINUTES=60
PROFILE=standard  # smoke, standard, or heavy

# Set repeat count based on profile
if [[ "$PROFILE" == "smoke" ]]; then
  REPEAT=20
elif [[ "$PROFILE" == "heavy" ]]; then
  REPEAT=200
else
  REPEAT=80
fi

# Create output directory
mkdir -p tmp/soak

# Run soak loop
END=$(( $(date +%s) + (MINUTES*60) ))
i=0
while [[ $(date +%s) -lt $END ]]; do
  i=$((i+1))
  echo "=== Iteration $i ===" | tee -a tmp/soak/soak.log
  cargo test -p ippan-consensus-dlc --test fairness_invariants -- --nocapture 2>&1 | tee -a tmp/soak/soak.log
  if [[ $i -ge $REPEAT ]]; then
    echo "Reached repeat cap ($REPEAT) for profile=$PROFILE" | tee -a tmp/soak/soak.log
    break
  fi
done
```

**Note**: Local runs may take hours. Use `smoke` profile for quick validation.

## Artifacts

### Location

After a workflow run completes:

1. Go to **Actions** → select the workflow run
2. Scroll to **Artifacts** section
3. Download `soak-dlc-longrun-<run_id>`

### Contents

- `meta.txt`: Run metadata (duration, profile, commit SHA, timestamp)
- `soak.log`: Full test output with all iterations

### Reading Artifacts

**Check for failures:**
```bash
grep -i "fail\|error\|panic" soak.log
```

**Count iterations:**
```bash
grep -c "=== Iteration" soak.log
```

**Check completion:**
```bash
tail -5 soak.log  # Should show "Soak completed OK"
```

**Review specific iteration:**
```bash
# Find iteration N
awk '/=== Iteration N ===/,/=== Iteration/ {print}' soak.log
```

## What Gets Tested

The soak runs `crates/consensus_dlc/tests/fairness_invariants.rs`, which validates:

- Primary/shadow verifier balance over 240 rounds
- Role fairness for honest validators
- Bounded adversarial selection
- Deterministic fairness scoring
- Long-run stability of DLC consensus

## Troubleshooting

**Workflow times out:**
- Reduce `minutes` input
- Use `smoke` profile

**Tests fail intermittently:**
- Check `soak.log` for patterns
- Review specific failing iterations
- Consider memory/resource constraints

**Artifacts missing:**
- Artifacts are retained for 21 days
- Check if workflow run completed (even if failed)

