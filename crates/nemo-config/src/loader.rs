//! Configuration loader - main entry point for loading configurations.

use crate::error::ConfigError;
use crate::registry::SchemaRegistry;
use crate::resolver::{ConfigResolver, ResolveContext};
use crate::validator::{ConfigValidator, ValidationResult};
use crate::xml_parser::XmlParser;
use crate::Value;
use std::path::Path;
use std::sync::Arc;

/// Main entry point for loading and processing configuration files.
pub struct ConfigurationLoader {
    validator: ConfigValidator,
    resolver: ConfigResolver,
    #[allow(dead_code)]
    schema_registry: Arc<SchemaRegistry>,
}

impl ConfigurationLoader {
    /// Creates a new configuration loader.
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        ConfigurationLoader {
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
        self.load_xml_string(&content, &source_name, path.parent())
    }

    /// Loads configuration from an XML string.
    pub fn load_xml_string(
        &self,
        content: &str,
        source_name: &str,
        base_dir: Option<&Path>,
    ) -> Result<Value, ConfigError> {
        let mut parser = XmlParser::new().with_source_name(source_name);
        if let Some(dir) = base_dir {
            parser = parser.with_base_dir(dir);
        }

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
    fn test_load_xml_string() {
        let loader = create_test_loader();

        let content = r#"
        <nemo>
            <app title="XML App">
                <window title="Test" />
                <theme name="kanagawa" mode="dark" />
            </app>
            <script src="./scripts" />
            <layout type="stack">
                <label id="header" text="Hello XML" />
            </layout>
        </nemo>
        "#;

        let value = loader.load_xml_string(content, "test.xml", None).unwrap();
        let app = value.get("app").unwrap();
        assert_eq!(
            app.get("title"),
            Some(&Value::String("XML App".to_string()))
        );

        let layout = value.get("layout").unwrap();
        assert_eq!(
            layout.get("type"),
            Some(&Value::String("stack".to_string()))
        );
    }

    #[test]
    fn test_load_xml_with_variables() {
        let loader = create_test_loader();

        let content = r#"
        <nemo>
            <variable name="greeting" default="Hello World" />
            <layout type="stack">
                <label id="lbl" text="${var.greeting}" />
            </layout>
        </nemo>
        "#;

        let value = loader.load_xml_string(content, "test.xml", None).unwrap();
        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let lbl = components.get("lbl").unwrap();
        assert_eq!(
            lbl.get("text"),
            Some(&Value::String("Hello World".to_string()))
        );
    }
}
