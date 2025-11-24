use ippan_storage::{export_snapshot, import_snapshot, Account, SledStorage, Storage};
use ippan_types::{Amount, Block, ChainState, Transaction};
use tempfile::TempDir;

#[test]
fn snapshot_roundtrip_preserves_state() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("db");
    let storage = SledStorage::new(&db_path).expect("create storage");
    storage
        .set_network_id("ippan-devnet")
        .expect("set network id");

    let account = Account {
        address: [1u8; 32],
        balance: 42,
        nonce: 7,
    };
    storage
        .update_account(account.clone())
        .expect("store account");

    let tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_micro_ipn(1_000), 1);
    storage.store_transaction(tx.clone()).expect("store tx");

    let block = Block::new(vec![], vec![tx.clone()], 1, [9u8; 32]);
    storage.store_block(block.clone()).expect("store block");

    let mut chain_state = ChainState::new();
    chain_state.set_round(1);
    storage
        .update_chain_state(&chain_state)
        .expect("update chain state");

    let snapshot_dir = temp_dir.path().join("snapshot");
    let manifest = export_snapshot(&storage, &snapshot_dir, None).expect("export snapshot");

    let restore_dir = TempDir::new().expect("restore dir");
    let restore_db = restore_dir.path().join("db");
    let mut restored = SledStorage::new(&restore_db).expect("restore storage");
    restored
        .set_network_id("ippan-devnet")
        .expect("set restored network id");
    let restored_manifest = import_snapshot(&mut restored, &snapshot_dir).expect("import snapshot");

    assert_eq!(manifest.height, restored_manifest.height);
    let original_block = storage
        .get_block_by_height(manifest.height)
        .expect("block lookup")
        .expect("block exists");
    let restored_block = restored
        .get_block_by_height(manifest.height)
        .expect("restored block lookup")
        .expect("restored block exists");
    assert_eq!(original_block.hash(), restored_block.hash());

    let restored_account = restored
        .get_account(&account.address)
        .expect("restored account lookup")
        .expect("restored account exists");
    assert_eq!(account.balance, restored_account.balance);
    assert_eq!(account.nonce, restored_account.nonce);
}
