# IPPAN Crypto

## Overview
- Provides key management, hashing, Merkle proofs, and zero-knowledge helpers.
- Acts as the shared cryptographic toolbox for consensus, mempool, wallet, and security layers.
- Emphasizes deterministic primitives that stay reproducible across the network.

## Key Modules
- `hash_functions`: Blake3, SHA2, Keccak, and trait-based hashing interfaces.
- `commitment_schemes`: Pedersen commitments and confidential proof helpers.
- `merkle_trees`: construct and verify Merkle proofs for DAG and state roots.
- `validators`: confidential block and transaction validation logic.
- `key_management` and `signature_schemes`: Ed25519 key generation and signing.

## Integration Notes
- Reuse `KeyPair` for validator and wallet identities; persist keys with secure storage.
- Enforce confidential transaction rules with `validate_confidential_transaction` before block inclusion.
- Wrap hashing through the provided traits to keep future algorithm swaps manageable.
