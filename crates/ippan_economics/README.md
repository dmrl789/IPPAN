# IPPAN Economics — DAG-Fair Emission Framework

Deterministic round-based emission and fair distribution for IPPAN’s BlockDAG.

## Overview

IPPAN's **DAG-Fair Emission Framework** transforms traditional block-based mining into **time-anchored micro-rewards**.  
Unlike linear blockchains where block intervals define issuance, IPPAN's BlockDAG creates **thousands of micro-blocks per second within overlapping rounds**.  
Rewards are computed **per round** and distributed proportionally to validators based on their participation and performance.

This crate implements the core economics logic for IPPAN, providing:

- **Deterministic Emission** — Round-based emission with Bitcoin-style halving schedule  
- **Hard Cap Enforcement** — 21M IPN maximum supply with automatic clamping and burn  
- **DAG-Fair Distribution** — Role-weighted proportional distribution across validators  
- **Fee Management** — Configurable fee caps per round  
- **Precision** — Uses micro-IPN (μIPN) for exact calculations (no floating point errors)  
- **Parallel Simulation** — Multi-core simulation support via Rayon  
- **Governance Controls** — On-chain parameter updates via validator voting

---

## Key Features

- **Round-based emission:** Rewards calculated per round, not per block  
- **DAG-Fair distribution:** Proportional rewards based on validator participation  
- **Deterministic halving:** Bitcoin-style halving schedule with round precision  
- **Supply integrity:** Hard-capped 21 M IPN with automatic burn of excess  
- **Governance controls:** On-chain parameter updates via validator voting  
- **Comprehensive auditing:** Supply verification and integrity checks  

### Monetary Unit

- 1 IPN = 1,000,000 μIPN (micro-IPN)  
- All calculations use `u128` for micro-IPN precision  
- Constants and conversion helpers provided  

### Emission Schedule

- **Initial Reward:** 0.0001 IPN (100 μIPN) per round  
- **Halving Interval:** ~2 years (≈ 630 million rounds @ 10 rounds/s)  
- **Hard Cap:** 21,000,000 IPN  
- **Formula:** `R(t) = R₀ / 2^⌊t / Tₕ⌋`

### Distribution Logic

- **Role Weights:**  
  - Proposer = 1.2×  
  - Verifier = 1.0×  
  - Observer = 0×  
- **Fairness:** Proportional to weighted contributions (blocks × uptime)  
- **Fee Cap:** ≤ 10% of round emission  
- **Uptime Score:** Adjusts rewards by validator reliability  

### Reward Composition

| Component | Source | Description |
|------------|---------|-------------|
| Round Emission | 85% of emission schedule | Direct rewards to validators |
| Transaction Fees | 90% of collected fees | Direct fees to validators |
| AI Service Commissions | 10% of emission | Reserved for AI inference rewards |
| Network Dividend | 5% of emission + 10% of fees | Accumulated for periodic redistribution |

The network dividend accumulates over time and is redistributed periodically (e.g., weekly) based on validator uptime and reputation scores.

---

## Core Components

### EmissionEngine

Calculates per-round rewards with halving and hard-cap enforcement:

```rust
use ippan_economics::prelude::*;

let mut engine = EmissionEngine::new();
let reward = engine.calculate_round_reward(1000)?;
