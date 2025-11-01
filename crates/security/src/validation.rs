use parking_lot::RwLock;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// High-level validator for incoming data
///
/// Converts arbitrary serializable payloads into `serde_json::Value` and
/// evaluates a list of declarative [`ValidationRule`] constraints.
pub struct InputValidator {
    regex_cache: RwLock<HashMap<String, Arc<Regex>>>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidator {
    /// Create a new validator instance
    pub fn new() -> Self {
        Self {
            regex_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Validate a payload against a list of rules
    pub fn validate<T>(&self, data: &T, rules: &[ValidationRule]) -> Result<(), ValidationError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(data)
            .map_err(|e| ValidationError::Serialization(e.to_string()))?;

        let mut issues = Vec::new();
        for rule in rules {
            issues.extend(self.apply_rule(&value, rule));
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::Failed { errors: issues })
        }
    }

    fn apply_rule(&self, value: &Value, rule: &ValidationRule) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let field_value = lookup_field(value, &rule.field);

        for constraint in &rule.constraints {
            if let Some(issue) = self.check_constraint(&rule.field, field_value, constraint) {
                issues.push(issue);
            }
        }

        issues
    }

    fn check_constraint(
        &self,
        field: &str,
        value: Option<&Value>,
        constraint: &Constraint,
    ) -> Option<ValidationIssue> {
        match constraint {
            Constraint::Required { message } => {
                if value.is_none() || matches!(value, Some(Value::Null)) {
                    return Some(ValidationIssue::new(
                        field,
                        "required",
                        message,
                        format!("Field `{}` is required", field),
                    ));
                }
            }
            Constraint::NonEmpty { message } => {
                if let Some(val) = value {
                    if is_empty(val) {
                        return Some(ValidationIssue::new(
                            field,
                            "empty",
                            message,
                            format!("Field `{}` must not be empty", field),
                        ));
                    }
                } else {
                    return Some(ValidationIssue::new(
                        field,
                        "empty",
                        message,
                        format!("Field `{}` must not be empty", field),
                    ));
                }
            }
            Constraint::MinLength { min, message } => {
                if let Some(len) = value.and_then(extract_length) {
                    if len < *min {
                        return Some(ValidationIssue::new(
                            field,
                            "min_length",
                            message,
                            format!("Field `{}` must have length ≥ {}", field, min),
                        ));
                    }
                }
            }
            Constraint::MaxLength { max, message } => {
                if let Some(len) = value.and_then(extract_length) {
                    if len > *max {
                        return Some(ValidationIssue::new(
                            field,
                            "max_length",
                            message,
                            format!("Field `{}` must have length ≤ {}", field, max),
                        ));
                    }
                }
            }
            Constraint::MinValue { min, message } => {
                if let Some(num) = value.and_then(extract_number) {
                    if num < *min {
                        return Some(ValidationIssue::new(
                            field,
                            "min_value",
                            message,
                            format!("Field `{}` must be ≥ {}", field, min),
                        ));
                    }
                }
            }
            Constraint::MaxValue { max, message } => {
                if let Some(num) = value.and_then(extract_number) {
                    if num > *max {
                        return Some(ValidationIssue::new(
                            field,
                            "max_value",
                            message,
                            format!("Field `{}` must be ≤ {}", field, max),
                        ));
                    }
                }
            }
            Constraint::Range { min, max, message } => {
                if let Some(num) = value.and_then(extract_number) {
                    if num < *min || num > *max {
                        return Some(ValidationIssue::new(
                            field,
                            "range",
                            message,
                            format!("Field `{}` must be within [{}, {}]", field, min, max),
                        ));
                    }
                }
            }
            Constraint::Regex { pattern, message } => {
                if let Some(Value::String(text)) = value {
                    if !self.get_or_compile_regex(pattern).is_match(text) {
                        return Some(ValidationIssue::new(
                            field,
                            "pattern",
                            message,
                            format!("Field `{}` does not match pattern", field),
                        ));
                    }
                }
            }
            Constraint::InSet { values, message } => {
                if let Some(Value::String(text)) = value {
                    if !values.iter().any(|candidate| candidate == text) {
                        return Some(ValidationIssue::new(
                            field,
                            "inclusion",
                            message,
                            format!("Field `{}` must be one of {:?}", field, values),
                        ));
                    }
                }
            }
        }

        None
    }

    fn get_or_compile_regex(&self, pattern: &str) -> Arc<Regex> {
        if let Some(existing) = self.regex_cache.read().get(pattern) {
            return Arc::clone(existing);
        }

        let compiled = Regex::new(pattern).unwrap_or_else(|_| Regex::new("^$").unwrap());
        let compiled = Arc::new(compiled);
        self.regex_cache
            .write()
            .entry(pattern.to_string())
            .or_insert_with(|| Arc::clone(&compiled));
        compiled
    }
}

/// Declarative validation rule describing the constraints for a particular field
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub field: String,
    pub constraints: Vec<Constraint>,
}

impl ValidationRule {
    /// Create a new rule for a given field path
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            constraints: Vec::new(),
        }
    }

    /// Add an arbitrary constraint to the rule
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn required(self) -> Self {
        self.with_constraint(Constraint::Required { message: None })
    }

    pub fn non_empty(self) -> Self {
        self.with_constraint(Constraint::NonEmpty { message: None })
    }

    pub fn min_length(self, min: usize) -> Self {
        self.with_constraint(Constraint::MinLength { min, message: None })
    }

    pub fn max_length(self, max: usize) -> Self {
        self.with_constraint(Constraint::MaxLength { max, message: None })
    }

    pub fn min_value(self, min: f64) -> Self {
        self.with_constraint(Constraint::MinValue { min, message: None })
    }

    pub fn max_value(self, max: f64) -> Self {
        self.with_constraint(Constraint::MaxValue { max, message: None })
    }

    pub fn range(self, min: f64, max: f64) -> Self {
        self.with_constraint(Constraint::Range {
            min,
            max,
            message: None,
        })
    }

    pub fn pattern(self, pattern: impl Into<String>) -> Self {
        self.with_constraint(Constraint::Regex {
            pattern: pattern.into(),
            message: None,
        })
    }

    pub fn in_set(self, values: Vec<String>) -> Self {
        self.with_constraint(Constraint::InSet {
            values,
            message: None,
        })
    }
}

/// Supported constraint types for validation
#[derive(Debug, Clone)]
pub enum Constraint {
    Required {
        message: Option<String>,
    },
    NonEmpty {
        message: Option<String>,
    },
    MinLength {
        min: usize,
        message: Option<String>,
    },
    MaxLength {
        max: usize,
        message: Option<String>,
    },
    MinValue {
        min: f64,
        message: Option<String>,
    },
    MaxValue {
        max: f64,
        message: Option<String>,
    },
    Range {
        min: f64,
        max: f64,
        message: Option<String>,
    },
    Regex {
        pattern: String,
        message: Option<String>,
    },
    InSet {
        values: Vec<String>,
        message: Option<String>,
    },
}

impl Constraint {
    pub fn with_message(self, message: impl Into<String>) -> Self {
        let message = Some(message.into());
        match self {
            Constraint::Required { .. } => Constraint::Required { message },
            Constraint::NonEmpty { .. } => Constraint::NonEmpty { message },
            Constraint::MinLength { min, .. } => Constraint::MinLength { min, message },
            Constraint::MaxLength { max, .. } => Constraint::MaxLength { max, message },
            Constraint::MinValue { min, .. } => Constraint::MinValue { min, message },
            Constraint::MaxValue { max, .. } => Constraint::MaxValue { max, message },
            Constraint::Range { min, max, .. } => Constraint::Range { min, max, message },
            Constraint::Regex { pattern, .. } => Constraint::Regex { pattern, message },
            Constraint::InSet { values, .. } => Constraint::InSet { values, message },
        }
    }
}

/// Validation error returned when one or more constraints fail
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validation failed for {0} field(s)")]
    Failed { errors: Vec<ValidationIssue> },
    #[error("Failed to serialize input for validation: {0}")]
    Serialization(String),
}

impl ValidationError {
    pub fn errors(&self) -> &[ValidationIssue] {
        match self {
            ValidationError::Failed { errors } => errors.as_slice(),
            ValidationError::Serialization(_) => &[],
        }
    }
}

/// Detailed information about a single validation failure
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationIssue {
    pub field: String,
    pub code: String,
    pub message: String,
}

impl ValidationIssue {
    fn new(
        field: impl Into<String>,
        code: impl Into<String>,
        custom_message: &Option<String>,
        default: String,
    ) -> Self {
        Self {
            field: field.into(),
            code: code.into(),
            message: custom_message.clone().unwrap_or(default),
        }
    }
}

fn lookup_field<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }

    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            Value::Array(items) => {
                let idx = segment.parse::<usize>().ok()?;
                current = items.get(idx)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

fn extract_length(value: &Value) -> Option<usize> {
    match value {
        Value::String(s) => Some(s.chars().count()),
        Value::Array(arr) => Some(arr.len()),
        Value::Object(map) => Some(map.len()),
        _ => None,
    }
}

fn extract_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(arr) => arr.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestPayload {
        email: String,
        age: u32,
        tags: Vec<String>,
    }

    #[test]
    fn validates_successfully() {
        let payload = TestPayload {
            email: "user@example.com".into(),
            age: 30,
            tags: vec!["admin".into()],
        };

        let validator = InputValidator::new();
        let rules = vec![
            ValidationRule::new("email")
                .required()
                .pattern(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
                .min_length(5),
            ValidationRule::new("age")
                .required()
                .min_value(18.0)
                .max_value(120.0),
            ValidationRule::new("tags").required().non_empty(),
        ];

        let result = validator.validate(&payload, &rules);
        assert!(result.is_ok());
    }

    #[test]
    fn collects_multiple_errors() {
        let payload = TestPayload {
            email: "not-an-email".into(),
            age: 10,
            tags: Vec::new(),
        };

        let validator = InputValidator::new();
        let rules = vec![
            ValidationRule::new("email")
                .required()
                .pattern(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
                .min_length(10),
            ValidationRule::new("age").min_value(18.0),
            ValidationRule::new("tags").non_empty(),
        ];

        let result = validator.validate(&payload, &rules);
        assert!(result.is_err());

        let errors = match result {
            Err(ValidationError::Failed { errors }) => errors,
            _ => panic!("Expected failed validation"),
        };

        assert!(errors.iter().any(|e| e.field == "email"));
        assert!(errors.iter().any(|e| e.field == "age"));
        assert!(errors.iter().any(|e| e.field == "tags"));
    }
}
