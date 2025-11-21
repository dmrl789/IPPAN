pub const IPPAN_VERSION_RC: &str = "v0.9.x-rc";
pub const IPPAN_VERSION_MAINNET: &str = "v1.0.0"; // planned
pub const IPPAN_NETWORK_MAINNET_ID: u32 = 1;
pub const IPPAN_NETWORK_TESTNET_ID: u32 = 100;
pub const IPPAN_VERSION: &str = "v0.9.0-rc1";

pub fn git_commit_hash() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("unknown")
}
