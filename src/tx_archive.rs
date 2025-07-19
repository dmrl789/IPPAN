use rocksdb::{DB, Options};

pub struct TxArchive {
    db: DB,
}

impl TxArchive {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).unwrap();
        TxArchive { db }
    }

    pub fn store_transaction(&self, tx_hash: &str, tx_data: &[u8]) {
        self.db.put(tx_hash, tx_data).unwrap();
    }

    pub fn store_file_manifest(&self, file_hash: &str, manifest: &[u8]) {
        self.db.put(file_hash, manifest).unwrap();
    }

    pub fn store_txt_record(&self, handle_type: &str, txt_payload: &[u8]) {
        self.db.put(handle_type, txt_payload).unwrap();
    }

    pub fn store_proof(&self, round_hash: &str, proof_payload: &[u8]) {
        self.db.put(round_hash, proof_payload).unwrap();
    }
} 