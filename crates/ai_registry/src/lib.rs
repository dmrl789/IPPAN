//! Minimal AI model registry interface for IPPAN.

/// Returns built-in model names known to this node.
pub fn builtin_model_names() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_is_empty_by_default() {
        let models = builtin_model_names();
        assert!(models.is_empty());
    }
}
