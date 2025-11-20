pub const IPPAN_VERSION: &str = "v0.9.0-rc1";

pub fn git_commit_hash() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("unknown")
}
