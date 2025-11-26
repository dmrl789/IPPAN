use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

const NUMERIC_SCALE: i128 = 1_000_000;

/// Validation error returned by [`InputValidator`].
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    /// The provided data could not be serialised to JSON for inspection.
    #[error("validation serialisation error: {0}")]
    Serialization(String),
    /// A required field was missing from the payload.
    #[error("missing required field `{0}`")]
    MissingField(String),
    /// Input data appears to contain a potentially malicious payload.
    #[error("potential injection detected in `{field}` (matched `{pattern}`)")]
    InjectionDetected { field: String, pattern: String },
    /// A specific validation rule failed for the supplied field.
    #[error("field `{field}` failed validation: {reason}")]
    RuleViolation { field: String, reason: String },
}

/// Validation rules that can be applied to incoming payloads.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Ensure that a field exists.
    RequiredField { field: String },
    /// Ensure that a string field is not empty once trimmed.
    NonEmptyString { field: String },
    /// Enforce a minimum string length.
    MinLength { field: String, min: usize },
    /// Enforce a maximum string length.
    MaxLength { field: String, max: usize },
    /// Ensure that a string value matches the supplied regular expression.
    Pattern { field: String, pattern: Regex },
    /// Limit the numeric range for a field. Bounds are inclusive when provided.
    NumericRange {
        field: String,
        min: Option<i128>,
        max: Option<i128>,
    },
    /// Restrict a field to a predefined whitelist of string values.
    AllowedValues { field: String, values: Vec<String> },
    /// Ensure an array field does not exceed the provided length.
    MaxArrayLength { field: String, max: usize },
}

/// Deterministic input validator used by the security layer to protect RPC endpoints.
#[derive(Debug, Default)]
pub struct InputValidator {
    blocklist_patterns: Vec<Regex>,
}

impl InputValidator {
    /// Create a new validator with a default set of sanitation patterns.
    pub fn new() -> Self {
        let mut patterns = Vec::new();
        // Common XSS vectors
        patterns.push(Regex::new(r"(?i)<script").unwrap());
        patterns.push(Regex::new(r"(?i)onerror\\s*=").unwrap());
        // Common SQL injection fragments
        patterns.push(Regex::new(r"(?i)union\\s+select").unwrap());
        patterns.push(Regex::new(r"(?i)drop\\s+table").unwrap());
        patterns.push(Regex::new(r"(?i)--\\s").unwrap());

        Self {
            blocklist_patterns: patterns,
        }
    }

    /// Validate input data against the supplied rules.
    pub fn validate<T>(&self, data: &T, rules: &[ValidationRule]) -> Result<(), ValidationError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(data)
            .map_err(|err| ValidationError::Serialization(err.to_string()))?;

        self.inspect_for_injection(&value, "")?;

        for rule in rules {
            self.apply_rule(rule, &value)?;
        }

        Ok(())
    }

    fn inspect_for_injection(&self, value: &Value, path: &str) -> Result<(), ValidationError> {
        match value {
            Value::String(s) => {
                for pattern in &self.blocklist_patterns {
                    if pattern.is_match(s) {
                        return Err(ValidationError::InjectionDetected {
                            field: if path.is_empty() {
                                "<root>".to_string()
                            } else {
                                path.to_string()
                            },
                            pattern: pattern.as_str().to_string(),
                        });
                    }
                }
            }
            Value::Array(items) => {
                for (idx, item) in items.iter().enumerate() {
                    let child_path = if path.is_empty() {
                        format!("[{idx}]")
                    } else {
                        format!("{path}[{idx}]")
                    };
                    self.inspect_for_injection(item, &child_path)?;
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.inspect_for_injection(item, &child_path)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn apply_rule(&self, rule: &ValidationRule, root: &Value) -> Result<(), ValidationError> {
        match rule {
            ValidationRule::RequiredField { field } => {
                let value = self.lookup_field(root, field)?;
                if value.is_null() {
                    return Err(ValidationError::MissingField(field.clone()));
                }
            }
            ValidationRule::NonEmptyString { field } => {
                let value = self.lookup_field(root, field)?;
                if let Value::String(s) = value {
                    if s.trim().is_empty() {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: "value cannot be empty".into(),
                        });
                    }
                } else {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected string".into(),
                    });
                }
            }
            ValidationRule::MinLength { field, min } => {
                let value = self.lookup_field(root, field)?;
                if let Value::String(s) = value {
                    if s.chars().count() < *min {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: format!("length must be at least {min} characters"),
                        });
                    }
                } else {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected string".into(),
                    });
                }
            }
            ValidationRule::MaxLength { field, max } => {
                let value = self.lookup_field(root, field)?;
                if let Value::String(s) = value {
                    if s.chars().count() > *max {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: format!("length must be at most {max} characters"),
                        });
                    }
                } else {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected string".into(),
                    });
                }
            }
            ValidationRule::Pattern { field, pattern } => {
                let value = self.lookup_field(root, field)?;
                if let Value::String(s) = value {
                    if !pattern.is_match(s) {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: "value does not match required pattern".into(),
                        });
                    }
                } else {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected string".into(),
                    });
                }
            }
            ValidationRule::NumericRange { field, min, max } => {
                let value = self.lookup_field(root, field)?;
                let number =
                    parse_numeric_value(value).ok_or_else(|| ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected numeric value".into(),
                    })?;

                if let Some(lower) = min {
                    if number < *lower {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: format!("value must be >= {}", format_numeric_value(*lower)),
                        });
                    }
                }

                if let Some(upper) = max {
                    if number > *upper {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: format!("value must be <= {}", format_numeric_value(*upper)),
                        });
                    }
                }
            }
            ValidationRule::AllowedValues { field, values } => {
                let value = self.lookup_field(root, field)?;
                let candidate = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: "unsupported type for allowed values".into(),
                        })
                    }
                };

                if !values.iter().any(|allowed| allowed == &candidate) {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: format!("value `{candidate}` is not allowed"),
                    });
                }
            }
            ValidationRule::MaxArrayLength { field, max } => {
                let value = self.lookup_field(root, field)?;
                if let Value::Array(items) = value {
                    if items.len() > *max {
                        return Err(ValidationError::RuleViolation {
                            field: field.clone(),
                            reason: format!("array length must be <= {max}"),
                        });
                    }
                } else {
                    return Err(ValidationError::RuleViolation {
                        field: field.clone(),
                        reason: "expected array".into(),
                    });
                }
            }
        }

        Ok(())
    }

    fn lookup_field<'a>(&self, root: &'a Value, path: &str) -> Result<&'a Value, ValidationError> {
        if path.is_empty() {
            return Ok(root);
        }

        let mut current = root;
        for segment in path.split('.') {
            match current {
                Value::Object(map) => {
                    current = map
                        .get(segment)
                        .ok_or_else(|| ValidationError::MissingField(path.to_string()))?;
                }
                _ => {
                    return Err(ValidationError::RuleViolation {
                        field: path.to_string(),
                        reason: "encountered non-object while traversing path".into(),
                    });
                }
            }
        }

        Ok(current)
    }
}

fn parse_numeric_value(value: &Value) -> Option<i128> {
    match value {
        Value::Number(n) => parse_decimal_str(&n.to_string()),
        Value::String(s) => parse_decimal_str(s),
        _ => None,
    }
}

fn parse_decimal_str(raw: &str) -> Option<i128> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };

    if digits.is_empty() {
        return None;
    }

    let mut parts = digits.split('.');
    let whole_part = parts.next().unwrap_or("");
    let fractional_part = parts.next();
    if parts.next().is_some() {
        return None;
    }

    let whole = if whole_part.is_empty() {
        0
    } else {
        whole_part.parse::<i128>().ok()?
    };

    let mut value = whole.checked_mul(NUMERIC_SCALE)?;
    if let Some(frac) = fractional_part {
        if !frac.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut fractional = frac.to_string();
        if fractional.len() > 6 {
            fractional.truncate(6);
        }
        while fractional.len() < 6 {
            fractional.push('0');
        }
        if !fractional.is_empty() {
            let frac_value = fractional.parse::<i128>().ok()?;
            value = value.checked_add(frac_value)?;
        }
    }

    if negative {
        value.checked_neg()
    } else {
        Some(value)
    }
}

fn format_numeric_value(value: i128) -> String {
    let negative = value < 0;
    let abs = value.abs();
    let whole = abs / NUMERIC_SCALE;
    let fractional = abs % NUMERIC_SCALE;

    if fractional == 0 {
        if negative {
            format!("-{whole}")
        } else {
            format!("{whole}")
        }
    } else {
        let mut frac_str = format!("{fractional:06}");
        while frac_str.ends_with('0') {
            frac_str.pop();
        }
        if negative {
            format!("-{whole}.{frac_str}")
        } else {
            format!("{whole}.{frac_str}")
        }
    }
}
