// In tests/wallet_tests.rs

use ippan_core::wallet::Wallet;

#[test]
fn test_wallet_generate_and_address() {
    let wallet = Wallet::generate();
    // Ensure address looks like a non-empty string
    assert!(!wallet.address.is_empty(), "Wallet address should not be empty");
    // Optionally, check hex is 64 chars, or whatever your format is:
    // assert_eq!(wallet.address.len(), 32);
}
