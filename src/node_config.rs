pub struct NodeConfig {
    pub archive_mode: bool,
    pub sync_target: Option<String>, // e.g., Some("https://api.ippan.net/txfeed")
    pub sync_interval_secs: u64,
    pub tx_types_to_sync: Vec<TxType>, // Optional filtering
}

impl NodeConfig {
    pub fn new(archive_mode: bool, sync_target: Option<String>, sync_interval_secs: u64, tx_types_to_sync: Vec<TxType>) -> Self {
        NodeConfig {
            archive_mode,
            sync_target,
            sync_interval_secs,
            tx_types_to_sync,
        }
    }
} 