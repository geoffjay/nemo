//! Configuration loader - main entry point for loading configurations.

use crate::error::ConfigError;
use crate::parser::HclParser;
use crate::registry::SchemaRegistry;
use crate::resolver::{ConfigResolver, ResolveContext};
use crate::validator::{ConfigValidator, ValidationResult};
use crate::Value;
use std::path::Path;
use std::sync::Arc;

/// Main entry point for loading and processing configuration files.
pub struct ConfigurationLoader {
    #[allow(dead_code)]
    parser: HclParser,
    validator: ConfigValidator,
    resolver: ConfigResolver,
    #[allow(dead_code)]
    schema_registry: Arc<SchemaRegistry>,
}

impl ConfigurationLoader {
    /// Creates a new configuration loader.
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        ConfigurationLoader {
            parser: HclParser::new(),
            validator: ConfigValidator::new(Arc::clone(&schema_registry)),
            resolver: ConfigResolver::new(),
            schema_registry,
        }
    }

    /// Loads a configuration file.
    pub fn load(&self, path: &Path) -> Result<Value, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

        let source_name = path.display().to_string();
        self.load_string(&content, &source_name)
    }

    /// Loads configuration from a string.
    pub fn load_string(&self, content: &str, source_name: &str) -> Result<Value, ConfigError> {
        // Parse
        let parser = HclParser::new().with_source_name(source_name);
        let raw_value = parser.parse(content).map_err(ConfigError::Parse)?;

        // Build resolve context from the parsed config
        let context = self.build_context(&raw_value);

        // Resolve expressions
        let resolved = self
            .resolver
            .resolve(raw_value, &context)
            .map_err(ConfigError::Resolve)?;

        Ok(resolved)
    }

    /// Loads and validates configuration against a schema.
    pub fn load_validated(&self, path: &Path, schema_name: &str) -> Result<Value, ConfigError> {
        let value = self.load(path)?;
        let result = self.validator.validate(&value, schema_name);

        if !result.valid {
            return Err(ConfigError::Validation {
                errors: result.errors,
            });
        }

        Ok(value)
    }

    /// Validates a configuration file.
    pub fn validate(&self, path: &Path, schema_name: &str) -> ValidationResult {
        match self.load(path) {
            Ok(value) => self.validator.validate(&value, schema_name),
            Err(e) => {
                let mut result = ValidationResult::default();
                result
                    .errors
                    .push(crate::ValidationError::load_error(&e.to_string()));
                result
            }
        }
    }

    /// Builds a resolve context from parsed configuration.
    fn build_context(&self, config: &Value) -> ResolveContext {
        let mut context = ResolveContext::with_system_env();
        context.config = config.clone();

        // Extract variables from "variable" blocks
        if let Some(variables) = config.get("variable").and_then(|v| v.as_object()) {
            for (name, var_config) in variables.iter() {
                if let Some(default) = var_config.get("default") {
                    context.variables.insert(name.clone(), default.clone());
                }
            }
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ConfigSchema, PropertySchema};

    fn create_test_loader() -> ConfigurationLoader {
        let registry = Arc::new(SchemaRegistry::new());

        let schema = ConfigSchema::new("application")
            .property("name", PropertySchema::string())
            .property("version", PropertySchema::string().with_default("1.0.0"))
            .require("name");

        registry.register(schema).unwrap();

        ConfigurationLoader::new(registry)
    }

    #[test]
    fn test_load_simple_config() {
        let loader = create_test_loader();

        let content = r#"
            name = "test"
            count = 42
        "#;

        let value = loader.load_string(content, "test.hcl").unwrap();
        assert_eq!(value.get("name"), Some(&Value::String("test".to_string())));
        assert_eq!(value.get("count"), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_load_with_variables() {
        let loader = create_test_loader();

        let content = r#"
            variable "app_name" {
                default = "My App"
            }

            name = "${var.app_name}"
        "#;

        let value = loader.load_string(content, "test.hcl").unwrap();
        assert_eq!(
            value.get("name"),
            Some(&Value::String("My App".to_string()))
        );
    }
}
