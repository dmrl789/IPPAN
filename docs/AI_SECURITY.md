# AI Security and Determinism Guarantees

This document describes the security measures and determinism guarantees for AI models used in IPPAN L1 consensus.

## Threat Model

### In-Scope Threats

1. **Non-Determinism**: Floating-point arithmetic causing divergent outputs across platforms  
2. **Model Tampering**: Malicious model substitution or corruption  
3. **Byzantine Models**: Adversarial models designed to manipulate validator selection  
4. **Gradient Attacks**: Models trained to favor specific validators  
5. **Overflow/Underflow**: Integer arithmetic bugs causing crashes or wrong outputs

### Out-of-Scope

- L2 AI attacks (handled separately)  
- Off-chain model training attacks (mitigated by governance review)  
- Side-channel attacks on model inference

## Determinism Guarantees

### Integer-Only Arithmetic

All model inference in L1 uses **integer-only arithmetic**. Floating-point operations are forbidden:

```rust
#![forbid(float_types)]
