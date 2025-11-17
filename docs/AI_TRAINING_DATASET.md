# AI Training Dataset Specification

This document defines the canonical dataset format consumed by the offline
`ai-trainer` crate. The trainer expects a deterministic, integer-only CSV file
where **each row describes a validator over a fixed telemetry window**.

## File format

* **Encoding:** UTF-8
* **Delimiter:** comma (`,`) with no quoted fields
* **Header:** required, must be the first non-comment line
* **Comments:** lines starting with `#` are ignored
* **Sorting:** rows MUST be sorted by `(validator_id, timestamp)` in ascending
  order. Any out-of-order row will cause the trainer to error.
* **Units:** all numeric values are signed 64-bit integers. Micros and counts
  must be pre-scaled; floating point columns are not allowed.

## Required columns

| Column | Description | Units |
|--------|-------------|-------|
| `validator_id` | Stable identifier for the validator | integer ID |
| `timestamp` | Start timestamp of the telemetry window | Unix seconds |
| `uptime_micros` | Time the validator reported healthy uptime | microseconds |
| `latency_micros` | Median observed consensus latency | microseconds |
| `votes_cast` | Number of votes cast during the window | count |
| `votes_missed` | Number of missed votes/slashes | count |
| `stake_atomic` | Total self-bonded stake | atomic IPN |
| `label` | Target fairness / priority label | integer score |

Only the feature columns (`uptime_micros`, `latency_micros`, `votes_cast`,
`votes_missed`, `stake_atomic`) are used as inputs during training, but the
trainer verifies that `validator_id` and `timestamp` exist and are sorted.

## Example dataset

```csv
validator_id,timestamp,uptime_micros,latency_micros,votes_cast,votes_missed,stake_atomic,label
1,1700000000,86400000000,120000,1500,5,520000000000,900000
1,1700000600,86350000000,110000,1520,4,520000000000,910000
2,1700000000,85000000000,150000,1400,20,500000000000,780000
2,1700000600,85100000000,140000,1410,18,500000000000,790000
3,1700000000,83000000000,130000,1600,2,600000000000,950000
3,1700000600,83100000000,125000,1605,2,600000000000,940000
```

The above sample is checked into the repository at
`data/ai_training/sample_validator_telemetry.csv` and can be used to verify the
trainer pipeline end-to-end.

## Validation rules

1. **No missing columns** – all required headers must be present.
2. **Deterministic sorting** – the `(validator_id, timestamp)` tuple must be
   strictly increasing or equal; otherwise the loader aborts.
3. **Integer enforcement** – parsing fails if any cell contains a float or
   non-numeric value.
4. **Consistent row width** – every line must have the same column count as the
   header.
5. **Deterministic shuffling** – the trainer uses a deterministic hash-based
   shuffle internally when a seed is provided, so identical datasets yield the
   same ordering on every run.

By enforcing the above constraints we guarantee that all nodes training from a
shared dataset will obtain **bit-identical models**, which is a prerequisite for
D-GBDT fairness across the network.
