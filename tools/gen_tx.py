#!/usr/bin/env python3
"""
Transaction generator for IPPAN TestSprite tests
Creates signed transactions in the format expected by the API
"""

import json
import sys
import hashlib
import time
from typing import Dict, Any

def create_canonical_transaction(from_addr: str, to_addr: str, amount: str, fee: str, nonce: int) -> Dict[str, Any]:
    """Create a canonical transaction JSON"""
    timestamp = str(int(time.time()))
    
    return {
        "chain_id": "ippan-devnet-001",
        "from": from_addr,
        "to": to_addr,
        "amount": amount,
        "fee": fee,
        "nonce": nonce,
        "timestamp": timestamp
    }

def sign_transaction(tx: Dict[str, Any]) -> str:
    """Sign transaction with a test signature (placeholder)"""
    # In a real implementation, this would use ed25519
    # For now, create a deterministic signature based on transaction content
    tx_json = json.dumps(tx, sort_keys=True, separators=(',', ':'))
    signature = hashlib.sha256(tx_json.encode()).hexdigest()
    return signature

def create_signed_transaction(from_addr: str, to_addr: str, amount: str, fee: str, nonce: int) -> Dict[str, Any]:
    """Create a signed transaction"""
    tx = create_canonical_transaction(from_addr, to_addr, amount, fee, nonce)
    signature = sign_transaction(tx)
    
    return {
        **tx,
        "signature": signature,
        "pubkey": "test_pubkey_hex_placeholder"
    }

def main():
    if len(sys.argv) < 6:
        print("Usage: gen_tx.py <from> <to> <amount> <fee> <nonce> [output_file]")
        sys.exit(1)
    
    from_addr = sys.argv[1]
    to_addr = sys.argv[2]
    amount = sys.argv[3]
    fee = sys.argv[4]
    nonce = int(sys.argv[5])
    output_file = sys.argv[6] if len(sys.argv) > 6 else "/tmp/tx.json"
    
    signed_tx = create_signed_transaction(from_addr, to_addr, amount, fee, nonce)
    
    with open(output_file, 'w') as f:
        json.dump(signed_tx, f, indent=2)
    
    print(f"Generated signed transaction: {output_file}")
    print(json.dumps(signed_tx, indent=2))

if __name__ == "__main__":
    main()

