//! Configuration validation.

use crate::error::ValidationError;
use crate::path::ConfigPath;
use crate::registry::SchemaRegistry;
use crate::schema::{ConfigSchema, PropertySchema, ValidationRule, ValueType};
use crate::Value;
use std::sync::Arc;

/// Validator for configuration values.
pub struct ConfigValidator {
    schema_registry: Arc<SchemaRegistry>,
}

impl ConfigValidator {
    /// Creates a new validator with the given schema registry.
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        ConfigValidator { schema_registry }
    }

    /// Validates a value against a named schema.
    pub fn validate(&self, value: &Value, schema_name: &str) -> ValidationResult {
        let mut result = ValidationResult::default();

        let schema = match self.schema_registry.get(schema_name) {
            Some(s) => s,
            None => {
                result
                    .errors
                    .push(ValidationError::schema_not_found(schema_name));
                return result;
            }
        };

        self.validate_against_schema(value, &schema, ConfigPath::root(), &mut result);
        result.valid = result.errors.is_empty();
        result
    }

    /// Validates a value against a schema.
    fn validate_against_schema(
        &self,
        value: &Value,
        schema: &ConfigSchema,
        path: ConfigPath,
        result: &mut ValidationResult,
    ) {
        let obj = match value {
            Value::Object(obj) => obj,
            _ => {
                result.errors.push(ValidationError::type_mismatch(
                    path,
                    "object",
                    value.type_name(),
                ));
                return;
            }
        };

        // Check required fields
        for required in &schema.required {
            if !obj.contains_key(required) {
                result
                    .errors
                    .push(ValidationError::missing_required(path.clone(), required));
            }
        }

        // Validate properties
        for (key, prop_value) in obj.iter() {
            let prop_path = path.join_key(key);

            if let Some(prop_schema) = schema.properties.get(key) {
                self.validate_property(prop_value, prop_schema, prop_path, result);
            } else if !schema.additional_properties {
                result.errors.push(ValidationError::unknown_property(
                    prop_path,
                    key,
                    schema.properties.keys().cloned().collect(),
                ));
            }
        }
    }

    /// Validates a property value.
    fn validate_property(
        &self,
        value: &Value,
        schema: &PropertySchema,
        path: ConfigPath,
        result: &mut ValidationResult,
    ) {
        // Check type
        if !self.type_matches(value, &schema.value_type) {
            result.errors.push(ValidationError::type_mismatch(
                path.clone(),
                &schema.value_type.to_string(),
                value.type_name(),
            ));
            return;
        }

        // Validate rules
        for rule in &schema.rules {
            if let Err(msg) = self.validate_rule(value, rule) {
                result
                    .errors
                    .push(ValidationError::rule_violation(path.clone(), rule, &msg));
            }
        }

        // Validate nested objects
        if let (Value::Object(_), Some(nested_schema)) = (value, &schema.nested) {
            self.validate_against_schema(value, nested_schema, path.clone(), result);
        }

        // Validate array items
        if let (Value::Array(arr), Some(item_schema)) = (value, &schema.items) {
            for (i, item) in arr.iter().enumerate() {
                let item_path = path.join_index(i);
                self.validate_property(item, item_schema, item_path, result);
            }
        }
    }

    /// Checks if a value matches an expected type.
    fn type_matches(&self, value: &Value, expected: &ValueType) -> bool {
        match expected {
            ValueType::Any => true,
            ValueType::String => matches!(value, Value::String(_)),
            ValueType::Integer => matches!(value, Value::Integer(_)),
            ValueType::Float => matches!(value, Value::Float(_) | Value::Integer(_)),
            ValueType::Boolean => matches!(value, Value::Bool(_)),
            ValueType::Array => matches!(value, Value::Array(_)),
            ValueType::Object => matches!(value, Value::Object(_)),
        }
    }

    /// Validates a value against a rule.
    fn validate_rule(&self, value: &Value, rule: &ValidationRule) -> Result<(), String> {
        match rule {
            ValidationRule::Min(min) => {
                if let Some(n) = value.as_i64() {
                    if n < *min {
                        return Err(format!("value {} is less than minimum {}", n, min));
                    }
                }
            }
            ValidationRule::Max(max) => {
                if let Some(n) = value.as_i64() {
                    if n > *max {
                        return Err(format!("value {} is greater than maximum {}", n, max));
                    }
                }
            }
            ValidationRule::MinLength(min) => {
                let len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(a) => a.len(),
                    _ => return Ok(()),
                };
                if len < *min {
                    return Err(format!("length {} is less than minimum {}", len, min));
                }
            }
            ValidationRule::MaxLength(max) => {
                let len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(a) => a.len(),
                    _ => return Ok(()),
                };
                if len > *max {
                    return Err(format!("length {} is greater than maximum {}", len, max));
                }
            }
            ValidationRule::Pattern(pattern) => {
                if let Value::String(s) = value {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| format!("invalid pattern: {}", e))?;
                    if !re.is_match(s) {
                        return Err(format!("value does not match pattern {}", pattern));
                    }
                }
            }
            ValidationRule::OneOf(allowed) => {
                if !allowed.contains(value) {
                    return Err(format!("value must be one of {:?}", allowed));
                }
            }
            ValidationRule::Custom { message, .. } => {
                return Err(message.clone());
            }
        }
        Ok(())
    }
}

/// Result of validation.
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Whether the validation passed.
    pub valid: bool,
    /// Validation errors.
    pub errors: Vec<ValidationError>,
    /// Validation warnings.
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn create_test_registry() -> Arc<SchemaRegistry> {
        let registry = Arc::new(SchemaRegistry::new());

        let schema = ConfigSchema::new("application")
            .property("name", PropertySchema::string())
            .property("port", PropertySchema::integer().min(1).max(65535))
            .require("name");

        registry.register(schema).unwrap();
        registry
    }

    #[test]
    fn test_validate_valid_config() {
        let registry = create_test_registry();
        let validator = ConfigValidator::new(registry);

        let mut obj = IndexMap::new();
        obj.insert("name".to_string(), Value::String("test".into()));
        obj.insert("port".to_string(), Value::Integer(8080));
        let value = Value::Object(obj);

        let result = validator.validate(&value, "application");
        assert!(result.valid);
    }

    #[test]
    fn test_validate_missing_required() {
        let registry = create_test_registry();
        let validator = ConfigValidator::new(registry);

        let obj = IndexMap::new();
        let value = Value::Object(obj);

        let result = validator.validate(&value, "application");
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("name")));
    }

    #[test]
    fn test_validate_rule_violation() {
        let registry = create_test_registry();
        let validator = ConfigValidator::new(registry);

        let mut obj = IndexMap::new();
        obj.insert("name".to_string(), Value::String("test".into()));
        obj.insert("port".to_string(), Value::Integer(99999));
        let value = Value::Object(obj);

        let result = validator.validate(&value, "application");
        assert!(!result.valid);
    }
}
