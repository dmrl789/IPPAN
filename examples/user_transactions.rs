//! User Transaction Examples for IPPAN
//! 
//! This file demonstrates how to use all 23 canonical transaction types
//! that users can submit to the IPPAN network.

use ippan::transaction_types::*;
use serde_json;

/// Example: Payment transaction
pub fn example_pay_transaction() -> PayTransaction {
    PayTransaction {
        from: "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        to: "@alice.ipn".to_string(),
        amount_ipn: "1.25000000".to_string(),
        memo: Some("invoice #42".to_string()),
        fee: "0.01250000".to_string(), // 1% of 1.25 IPN = 0.0125 IPN
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Batch payment transaction
pub fn example_pay_batch_transaction() -> PayBatchTransaction {
    PayBatchTransaction {
        from: "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        items: vec![
            PaymentItem {
                to: "@a.ipn".to_string(),
                amount_ipn: "0.50000000".to_string(),
                memo: None,
            },
            PaymentItem {
                to: "@b.ipn".to_string(),
                amount_ipn: "0.20000000".to_string(),
                memo: Some("partial payment".to_string()),
            },
        ],
        fee: "0.00000050".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Invoice creation transaction
pub fn example_invoice_create_transaction() -> InvoiceCreateTransaction {
    InvoiceCreateTransaction {
        to: "@me.ipn".to_string(),
        amount_ipn: "12.34000000".to_string(),
        reference: Some("ORD-9931".to_string()),
        expires_at_us: Some(1756000000000000),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Handle registration transaction
pub fn example_handle_register_transaction() -> HandleRegisterTransaction {
    HandleRegisterTransaction {
        handle: "@desiree.ipn".to_string(),
        owner_pk: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        years: 1,
        fee: "0.50000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Handle renewal transaction
pub fn example_handle_renew_transaction() -> HandleRenewTransaction {
    HandleRenewTransaction {
        handle: "@desiree.ipn".to_string(),
        years: 1,
        fee: "0.50000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Handle transfer transaction
pub fn example_handle_transfer_transaction() -> HandleTransferTransaction {
    HandleTransferTransaction {
        handle: "@desiree.ipn".to_string(),
        new_owner_pk: "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        fee: "0.01000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Handle update transaction
pub fn example_handle_update_transaction() -> HandleUpdateTransaction {
    HandleUpdateTransaction {
        handle: "@desiree.ipn".to_string(),
        nonce: 42,
        ops: vec![
            HandleUpdateOp {
                op: "PATCH".to_string(),
                path: "addresses".to_string(),
                value: Some(serde_json::json!({
                    "ippan": "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                })),
            },
            HandleUpdateOp {
                op: "SET".to_string(),
                path: "content".to_string(),
                value: Some(serde_json::json!("dht:0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")),
            },
        ],
        ttl_ms: 3600000,
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Domain registration transaction
pub fn example_domain_register_transaction() -> DomainRegisterTransaction {
    DomainRegisterTransaction {
        domain: "example.ipn".to_string(),
        owner_pk: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        years: 1,
        plan: "standard".to_string(),
        fee: "8.00000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Domain renewal transaction
pub fn example_domain_renew_transaction() -> DomainRenewTransaction {
    DomainRenewTransaction {
        domain: "example.ipn".to_string(),
        years: 1,
        fee: "8.00000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Domain transfer transaction
pub fn example_domain_transfer_transaction() -> DomainTransferTransaction {
    DomainTransferTransaction {
        domain: "example.ipn".to_string(),
        new_owner_pk: "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        fee: "0.05000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Zone update transaction (DNS records)
pub fn example_zone_update_transaction() -> ZoneUpdateTransaction {
    ZoneUpdateTransaction {
        domain: "example.ipn".to_string(),
        nonce: 12,
        ops: vec![
            ZoneOp {
                op: "UPSERT_RRSET".to_string(),
                name: Some("www".to_string()),
                rtype: Some("A".to_string()),
                ttl: Some(300),
                records: Some(vec!["93.184.216.34".to_string()]),
            },
            ZoneOp {
                op: "UPSERT_RRSET".to_string(),
                name: Some("@".to_string()),
                rtype: Some("ALIAS".to_string()),
                ttl: Some(300),
                records: Some(vec!["root.host.net.".to_string()]),
            },
            ZoneOp {
                op: "UPSERT_RRSET".to_string(),
                name: Some("mail".to_string()),
                rtype: Some("MX".to_string()),
                ttl: Some(300),
                records: Some(vec!["10 mail.example.com.".to_string()]),
            },
        ],
        updated_at_us: 1755327600123456,
        fee_nano: 100,
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: File publish transaction
pub fn example_file_publish_transaction() -> FilePublishTransaction {
    FilePublishTransaction {
        publisher: "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        hash_timer: "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        size_bytes: 1048576,
        mime: "image/png".to_string(),
        replicas: 3,
        storage_plan: "paid".to_string(),
        fee: "0.00000100".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: File metadata update transaction
pub fn example_file_update_metadata_transaction() -> FileUpdateMetadataTransaction {
    FileUpdateMetadataTransaction {
        hash_timer: "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        ops: vec![
            MetadataUpdateOp {
                op: "SET".to_string(),
                path: "title".to_string(),
                value: Some(serde_json::json!("Logo v2")),
            },
            MetadataUpdateOp {
                op: "PATCH".to_string(),
                path: "tags".to_string(),
                value: Some(serde_json::json!(["logo", "branding", "v2"])),
            },
        ],
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Storage rent topup transaction
pub fn example_storage_rent_topup_transaction() -> StorageRentTopupTransaction {
    StorageRentTopupTransaction {
        hash_timer: "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        amount_ipn: "3.00000000".to_string(),
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Pin request transaction
pub fn example_pin_request_transaction() -> PinRequestTransaction {
    PinRequestTransaction {
        hash_timer: "0x8fb5abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        replicas: 2,
        max_price_ipn: "0.10000000".to_string(),
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Stake bond transaction
pub fn example_stake_bond_transaction() -> StakeBondTransaction {
    StakeBondTransaction {
        validator_pk: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        amount_ipn: "10000.00000000".to_string(),
        min_lock_days: 30,
        fee: "0.01000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Stake unbond transaction
pub fn example_stake_unbond_transaction() -> StakeUnbondTransaction {
    StakeUnbondTransaction {
        validator_pk: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        amount_ipn: "5000.00000000".to_string(),
        fee: "0.01000000".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Stake withdraw transaction
pub fn example_stake_withdraw_transaction() -> StakeWithdrawTransaction {
    StakeWithdrawTransaction {
        validator_pk: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Faucet claim transaction
pub fn example_faucet_claim_transaction() -> FaucetClaimTransaction {
    FaucetClaimTransaction {
        handle_or_addr: "@newuser.ipn".to_string(),
        uptime_proof: "proof_data_here".to_string(),
        fee: "0".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Key rotation transaction
pub fn example_key_rotate_transaction() -> KeyRotateTransaction {
    KeyRotateTransaction {
        address: "i1abc1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        new_owner_pk: "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Set controllers transaction
pub fn example_set_controllers_transaction() -> SetControllersTransaction {
    SetControllersTransaction {
        target_type: "domain".to_string(),
        target_id: "example.ipn".to_string(),
        controllers: vec![
            "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            "ed25519:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        ],
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Governance vote transaction
pub fn example_gov_vote_transaction() -> GovVoteTransaction {
    GovVoteTransaction {
        proposal_id: "P-2025-08-17-01".to_string(),
        choice: "yes".to_string(),
        stake_weight: None,
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Example: Service payment transaction
pub fn example_service_pay_transaction() -> ServicePayTransaction {
    ServicePayTransaction {
        service_id: "cdn.tier2".to_string(),
        plan: "monthly".to_string(),
        amount_ipn: "2.50000000".to_string(),
        period: Some("monthly".to_string()),
        fee: "0.00000010".to_string(),
        sig: "ed25519:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    }
}

/// Generate all example transactions and print them as JSON
pub fn print_all_examples() {
    println!("=== IPPAN User Transaction Examples ===\n");
    
    // Payments
    println!("1. Pay Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_pay_transaction()).unwrap());
    println!();
    
    println!("2. Pay Batch Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_pay_batch_transaction()).unwrap());
    println!();
    
    println!("3. Invoice Create Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_invoice_create_transaction()).unwrap());
    println!();
    
    // Handles
    println!("4. Handle Register Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_handle_register_transaction()).unwrap());
    println!();
    
    println!("5. Handle Renew Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_handle_renew_transaction()).unwrap());
    println!();
    
    println!("6. Handle Transfer Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_handle_transfer_transaction()).unwrap());
    println!();
    
    println!("7. Handle Update Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_handle_update_transaction()).unwrap());
    println!();
    
    // Domains & DNS
    println!("8. Domain Register Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_domain_register_transaction()).unwrap());
    println!();
    
    println!("9. Domain Renew Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_domain_renew_transaction()).unwrap());
    println!();
    
    println!("10. Domain Transfer Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_domain_transfer_transaction()).unwrap());
    println!();
    
    println!("11. Zone Update Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_zone_update_transaction()).unwrap());
    println!();
    
    // Storage
    println!("12. File Publish Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_file_publish_transaction()).unwrap());
    println!();
    
    println!("13. File Update Metadata Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_file_update_metadata_transaction()).unwrap());
    println!();
    
    println!("14. Storage Rent Topup Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_storage_rent_topup_transaction()).unwrap());
    println!();
    
    println!("15. Pin Request Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_pin_request_transaction()).unwrap());
    println!();
    
    // Staking
    println!("16. Stake Bond Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_stake_bond_transaction()).unwrap());
    println!();
    
    println!("17. Stake Unbond Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_stake_unbond_transaction()).unwrap());
    println!();
    
    println!("18. Stake Withdraw Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_stake_withdraw_transaction()).unwrap());
    println!();
    
    // Faucet
    println!("19. Faucet Claim Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_faucet_claim_transaction()).unwrap());
    println!();
    
    // Account Management
    println!("20. Key Rotate Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_key_rotate_transaction()).unwrap());
    println!();
    
    println!("21. Set Controllers Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_set_controllers_transaction()).unwrap());
    println!();
    
    // Governance
    println!("22. Governance Vote Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_gov_vote_transaction()).unwrap());
    println!();
    
    // Service Payments
    println!("23. Service Pay Transaction:");
    println!("{}", serde_json::to_string_pretty(&example_service_pay_transaction()).unwrap());
    println!();
}

/// Print CLI command examples
pub fn print_cli_examples() {
    println!("=== IPPAN CLI Command Examples ===\n");
    
    println!("# Send IPN");
    println!("ippan-cli pay --from i1abc... --to @alice.ipn --amount 1.25 --memo \"invoice #42\" --fee auto --key-file keys/me.json\n");
    
    println!("# Buy a domain");
    println!("ippan-cli domain-register --domain example.ipn --years 1 --plan standard --owner-pk keys/me.pub --fee auto --key-file keys/me.json\n");
    
    println!("# Buy a handle");
    println!("ippan-cli handle-register --handle @desiree.ipn --years 1 --owner-pk keys/me.pub --fee auto --key-file keys/me.json\n");
    
    println!("# Set DNS data (zone update)");
    println!("ippan-cli zone-update --domain example.ipn --nonce 12 --ops-file dns_ops.json --fee-nano auto --key-file keys/me.json\n");
    
    println!("# Set handle data");
    println!("ippan-cli handle-update --handle @desiree.ipn --nonce 42 --ops-file handle_ops.json --ttl-ms 3600000 --fee auto --key-file keys/me.json\n");
    
    println!("# Publish a file to CDN/DHT");
    println!("ippan-cli file-publish --publisher i1abc... --file-path ./logo.png --replicas 3 --storage-plan paid --fee auto --key-file keys/me.json\n");
    
    println!("# Stake to become validator");
    println!("ippan-cli stake-bond --validator-pk keys/validator.pub --amount 10000 --min-lock-days 30 --fee auto --key-file keys/me.json\n");
    
    println!("# Claim from faucet");
    println!("ippan-cli faucet-claim --handle-or-addr @newuser.ipn --uptime-proof proof_data --fee 0 --key-file keys/me.json\n");
}

/// Main function to run examples
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("IPPAN User Transaction Examples\n");
    
    // Print CLI examples
    print_cli_examples();
    println!();
    
    // Print JSON examples
    print_all_examples();
    
    Ok(())
}
