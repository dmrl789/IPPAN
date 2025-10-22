//! Core AI utilities and determinism helpers for IPPAN.

/// Returns a new vector sorted in a deterministic (stable) way.
pub fn deterministically_sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    // Rust's sort is deterministic for a given input and ordering.
    items.sort();
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_is_deterministic_for_integers() {
        let input = vec![3, 1, 2, 2, 5, 4];
        let out1 = deterministically_sorted(input.clone());
        let out2 = deterministically_sorted(input);
        assert_eq!(out1, out2);
        assert_eq!(out1, vec![1, 2, 2, 3, 4, 5]);
    }
}
