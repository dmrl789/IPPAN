use crate::types::MicroIPN;

/// Verify that the total minted over an epoch equals the deterministic sum.
/// Return the excess (if any) that must be auto-burned to maintain the hard cap.
///
/// - `expected_epoch_emission_micro`: sum of R(t) over the epoch rounds (μIPN)
/// - `actual_minted_micro`: sum actually minted on chain (μIPN)
///
/// If `actual` > `expected`, the difference should be burned.
/// If `actual` <= `expected`, returns 0 (no burn).
pub fn epoch_auto_burn(
    expected_epoch_emission_micro: MicroIPN,
    actual_minted_micro: MicroIPN,
) -> MicroIPN {
    actual_minted_micro.saturating_sub(expected_epoch_emission_micro)
}

/// Sum helper to compute expected emission for a closed interval of rounds [start, end] inclusive.
/// Caller should clamp per-round values to hard cap before summation (if needed).
pub fn sum_emission_over_rounds<F>(
    start: u64,
    end: u64,
    mut emission_fn: F,
) -> MicroIPN
where
    F: FnMut(u64) -> MicroIPN,
{
    let mut acc: MicroIPN = 0;
    for r in start..=end {
        acc = acc.saturating_add(emission_fn(r));
    }
    acc
}
