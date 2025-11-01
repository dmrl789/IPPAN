use regex::RegexBuilder;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

/// Validation rules that can be applied to serialized data.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Value must not be empty (applies to strings, arrays, and maps).
    NonEmpty,
    /// Minimum length (string length in characters or array length).
    MinLength(usize),
    /// Maximum length (string length in characters or array length).
    MaxLength(usize),
    /// Value must match the provided regular expression pattern.
    Pattern {
        /// Pattern expressed as a regular expression.
        pattern: String,
        /// Perform case insensitive matching.
        case_insensitive: bool,
    },
    /// Value must be one of the provided values.
    AllowedValues(Vec<Value>),
    /// Numeric minimum (inclusive).
    MinValue(f64),
    /// Numeric maximum (inclusive).
    MaxValue(f64),
}

/// Validation error raised when input fails to satisfy a rule.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("failed to serialise value for validation: {0}")]
    Serialization(String),
    #[error("validation rule `{rule}` failed: {reason}")]
    RuleViolation { rule: &'static str, reason: String },
}

/// Validates arbitrary serialisable inputs against a set of validation rules.
#[derive(Debug, Default)]
pub struct InputValidator;

impl InputValidator {
    /// Create a new validator instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate `data` against the supplied `rules`.
    pub fn validate<T>(&self, data: &T, rules: &[ValidationRule]) -> Result<(), ValidationError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(data)
            .map_err(|err| ValidationError::Serialization(err.to_string()))?;

        for rule in rules {
            rule.validate(&value)?;
        }

        Ok(())
    }
}

impl ValidationRule {
    fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        match self {
            ValidationRule::NonEmpty => {
                if is_empty(value) {
                    Err(ValidationError::RuleViolation {
                        rule: "non_empty",
                        reason: "value was empty".to_string(),
                    })
                } else {
                    Ok(())
                }
            }
            ValidationRule::MinLength(min) => {
                let len = value_length(value).ok_or_else(|| ValidationError::RuleViolation {
                    rule: "min_length",
                    reason: "rule only applies to strings and arrays".to_string(),
                })?;

                if len < *min {
                    Err(ValidationError::RuleViolation {
                        rule: "min_length",
                        reason: format!("length {len} is smaller than required minimum {min}"),
                    })
                } else {
                    Ok(())
                }
            }
            ValidationRule::MaxLength(max) => {
                let len = value_length(value).ok_or_else(|| ValidationError::RuleViolation {
                    rule: "max_length",
                    reason: "rule only applies to strings and arrays".to_string(),
                })?;

                if len > *max {
                    Err(ValidationError::RuleViolation {
                        rule: "max_length",
                        reason: format!("length {len} exceeds allowed maximum {max}"),
                    })
                } else {
                    Ok(())
                }
            }
            ValidationRule::Pattern {
                pattern,
                case_insensitive,
            } => {
                let string_value =
                    value_as_string(value).ok_or_else(|| ValidationError::RuleViolation {
                        rule: "pattern",
                        reason: "rule only applies to string-compatible values".to_string(),
                    })?;

                let regex = RegexBuilder::new(pattern)
                    .case_insensitive(*case_insensitive)
                    .build()
                    .map_err(|err| ValidationError::RuleViolation {
                        rule: "pattern",
                        reason: format!("invalid pattern: {err}"),
                    })?;

                if regex.is_match(&string_value) {
                    Ok(())
                } else {
                    Err(ValidationError::RuleViolation {
                        rule: "pattern",
                        reason: "value did not match required pattern".to_string(),
                    })
                }
            }
            ValidationRule::AllowedValues(options) => {
                if options.iter().any(|candidate| candidate == value) {
                    Ok(())
                } else {
                    Err(ValidationError::RuleViolation {
                        rule: "allowed_values",
                        reason: "value was not present in the allowed set".to_string(),
                    })
                }
            }
            ValidationRule::MinValue(min) => {
                let numeric =
                    value_to_f64(value).ok_or_else(|| ValidationError::RuleViolation {
                        rule: "min_value",
                        reason: "rule only applies to numeric-compatible values".to_string(),
                    })?;

                if numeric < *min {
                    Err(ValidationError::RuleViolation {
                        rule: "min_value",
                        reason: format!("value {numeric} is less than minimum {min}"),
                    })
                } else {
                    Ok(())
                }
            }
            ValidationRule::MaxValue(max) => {
                let numeric =
                    value_to_f64(value).ok_or_else(|| ValidationError::RuleViolation {
                        rule: "max_value",
                        reason: "rule only applies to numeric-compatible values".to_string(),
                    })?;

                if numeric > *max {
                    Err(ValidationError::RuleViolation {
                        rule: "max_value",
                        reason: format!("value {numeric} exceeds maximum {max}"),
                    })
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

fn value_length(value: &Value) -> Option<usize> {
    match value {
        Value::String(s) => Some(s.chars().count()),
        Value::Array(items) => Some(items.len()),
        Value::Object(map) => Some(map.len()),
        _ => None,
    }
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(num) => Some(num.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_rule_rejects_empty_string() {
        let validator = InputValidator::new();
        let result = validator.validate(&"", &[ValidationRule::NonEmpty]);
        assert!(matches!(result, Err(ValidationError::RuleViolation { .. })));
    }

    #[test]
    fn length_rules_apply_to_strings() {
        let validator = InputValidator::new();
        let rules = [ValidationRule::MinLength(2), ValidationRule::MaxLength(5)];
        assert!(validator.validate(&"ok", &rules).is_ok());
        assert!(validator.validate(&"a", &rules).is_err());
        assert!(validator.validate(&"toolong", &rules).is_err());
    }

    #[test]
    fn pattern_rule_validates_against_regex() {
        let validator = InputValidator::new();
        let rule = ValidationRule::Pattern {
            pattern: "^[a-z]{3}[0-9]{2}$".to_string(),
            case_insensitive: false,
        };
        assert!(validator.validate(&"abc12", &[rule.clone()]).is_ok());
        assert!(validator.validate(&"ABC12", &[rule]).is_err());
    }

    #[test]
    fn numeric_rules_support_strings_and_numbers() {
        let validator = InputValidator::new();
        let rules = [
            ValidationRule::MinValue(10.0),
            ValidationRule::MaxValue(20.0),
        ];
        assert!(validator.validate(&15, &rules).is_ok());
        assert!(validator.validate(&"9", &rules).is_err());
        assert!(validator.validate(&"21", &rules).is_err());
    }
}
