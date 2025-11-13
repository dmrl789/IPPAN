# IPPAN AI Trainer

Deterministic offline GBDT (Gradient Boosted Decision Tree) trainer for the IPPAN blockchain.

## Features

- **Fully Deterministic**: Produces identical models across all platforms and runs
- **Integer-Only Math**: No floating-point operations; uses fixed-point arithmetic (SCALE = 1,000,000)
- **Exact-Greedy CART**: Implements CART decision tree algorithm with exact split search
- **Gradient Boosting**: MSE loss with configurable learning rate
- **Quantized Features**: Configurable feature quantization for deterministic splits
- **Reproducible Hashing**: BLAKE3 hash of canonical JSON output

## Installation

```bash
cargo build -p ippan-ai-trainer --release
```

## Usage

### Training a Model

```bash
ippan-train \
  --input data/training.csv \
  --output models/gbdt \
  --trees 64 \
  --max-depth 6 \
  --min-samples-leaf 32 \
  --learning-rate 100000 \
  --quant-step 1000 \
  --seed 42
```

### CSV Format

The input CSV must contain integer-only values (already scaled):

```csv
feature1,feature2,feature3,target
100000,200000,300000,1000000
150000,250000,350000,2000000
200000,300000,400000,3000000
```

- All features must be integers (pre-scaled to fixed-point)
- Last column is the target value
- No header row required (but will be skipped if starts with `#`)

### CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input` | (required) | Input CSV dataset path |
| `--output` | `models/gbdt` | Output directory |
| `--trees` | `64` | Number of boosting trees |
| `--max-depth` | `6` | Maximum tree depth |
| `--min-samples-leaf` | `32` | Minimum samples per leaf node |
| `--learning-rate` | `100000` | Learning rate (fixed-point, 100000 = 0.1) |
| `--quant-step` | `1000` | Feature quantization step |
| `--seed` | `42` | Random seed for shuffling |
| `--scale` | `10000` | Output scale |
| `--no-shuffle` | `false` | Skip dataset shuffling |
| `--verbose` | `false` | Enable verbose logging |

## Output

Training produces two files:

1. **`active.json`** - Canonical JSON model file
2. **`active.hash`** - BLAKE3 hash (hex) of the model JSON

The model format is compatible with `ippan-ai-core::gbdt::GBDTModel`.

## Determinism Guarantees

The trainer ensures bit-for-bit reproducibility through:

1. **Fixed-Point Arithmetic**: All calculations use 64-bit integers with SCALE=1,000,000
2. **Deterministic Shuffling**: xxhash64-based row ordering with fixed seed
3. **Stable Tie-Breaking**: Splits with equal gain are ordered by (feature_idx, threshold, node_id)
4. **Quantized Thresholds**: Split candidates are quantized to `quant_step` intervals
5. **Canonical JSON**: Serialization produces identical byte sequences

## Testing

```bash
# Run all tests
cargo test -p ippan-ai-trainer

# Run with verbose output
cargo test -p ippan-ai-trainer -- --nocapture
```

### Test Coverage

- Unit tests for all modules (deterministic, dataset, cart, trainer)
- Integration tests verifying cross-run determinism
- Synthetic dataset tests for reproducibility

## Architecture

### Modules

- **`deterministic`** - LCG RNG, xxhash64, tie-breaking logic
- **`dataset`** - CSV loading and deterministic shuffling
- **`cart`** - CART decision tree builder with exact-greedy splits
- **`trainer`** - GBDT trainer with gradient boosting

### Algorithm

1. **Initialization**: Calculate bias (mean of targets)
2. **Boosting Loop**: For each tree:
   - Calculate gradients: `pred - target`
   - Calculate hessians: constant `1000`
   - Build CART tree with exact-greedy splits
   - Update predictions with scaled tree output
3. **Output**: Serialize model to canonical JSON + hash

### Split Selection

For each node:
1. Enumerate all features
2. Get quantized threshold candidates
3. Calculate split gain: `G_left²/H_left + G_right²/H_right - G_parent²/H_parent`
4. Select best split with deterministic tie-breaking

## Example

```rust
use ippan_ai_trainer::{Dataset, GbdtConfig, GbdtTrainer};

let dataset = Dataset::from_csv("data.csv")?;
let config = GbdtConfig {
    num_trees: 64,
    max_depth: 6,
    min_samples_leaf: 32,
    learning_rate: 100_000, // 0.1
    quant_step: 1000,
    scale: 10_000,
};

let trainer = GbdtTrainer::new(config);
let model = trainer.train(&dataset)?;

model.save_json("model.json")?;
```

## License

Apache-2.0

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
