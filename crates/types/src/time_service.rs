use std::convert::TryFrom;

pub use ippan_time::{
    generate_entropy, ingest_sample, init, now, now_us, sign_hashtimer, start_time_sync, status,
    verify_hashtimer, TimeSyncService,
};

/// Convenience function to get current IPPAN time in microseconds as `u64`.
pub fn ippan_time_now() -> u64 {
    now_us().max(0) as u64
}

/// Convenience function to initialize IPPAN time.
pub fn ippan_time_init() {
    init();
}

/// Convenience function to ingest a peer sample provided as `u64`.
pub fn ippan_time_ingest_sample(peer_time_us: u64) {
    let peer_time = i64::try_from(peer_time_us).unwrap_or(i64::MAX);
    ingest_sample(peer_time);
}

/// Spawn the background IPPAN time synchronization service on the provided address.
pub async fn ippan_time_start_sync(listen_addr: &str) {
    start_time_sync(listen_addr).await;
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::Duration;

    struct Guard<'a> {
        _inner: MutexGuard<'a, ()>,
    }

    fn lock_time_service() -> Guard<'static> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let lock = TEST_LOCK.get_or_init(|| Mutex::new(()));
        Guard {
            _inner: lock.lock().expect("test lock poisoned"),
        }
    }

    #[test]
    fn test_monotonic_wrapper_time() {
        let _guard = lock_time_service();
        ippan_time_init();

        let t1 = ippan_time_now();
        thread::sleep(Duration::from_millis(1));
        let t2 = ippan_time_now();

        assert!(t2 > t1);
    }

    #[test]
    fn test_ingest_sample_wrapper() {
        let _guard = lock_time_service();
        ippan_time_init();

        let base = ippan_time_now();
        ippan_time_ingest_sample(base + 500);
        let (_, offset, count) = status();

        assert!(count > 0);
        assert!(offset >= 0);
    }
}
