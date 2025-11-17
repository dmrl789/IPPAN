pub const IPPAN_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn git_commit_hash() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("unknown")
}
