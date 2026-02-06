//! Nemo Configuration Engine - HCL parsing and schema validation.
//!
//! This crate provides the configuration system for Nemo applications,
//! including HCL parsing, schema validation, and expression resolution.

mod error;
mod loader;
mod location;
mod parser;
mod path;
mod registry;
mod resolver;
mod schema;
mod validator;
mod value;

pub use error::{ConfigError, ErrorCode, ParseError, ResolveError, SchemaError, ValidationError};
pub use loader::ConfigurationLoader;
pub use location::SourceLocation;
pub use parser::HclParser;
pub use path::{ConfigPath, PathParseError, PathSegment};
pub use registry::SchemaRegistry;
pub use resolver::{ConfigFunction, ConfigResolver, ResolveContext};
pub use schema::{ConfigSchema, PropertySchema, ValidationRule, ValueType};
pub use validator::{ConfigValidator, ValidationResult};
pub use value::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_end_to_end() {
        let registry = Arc::new(SchemaRegistry::new());

        let schema = ConfigSchema::new("app")
            .property("name", PropertySchema::string())
            .property("port", PropertySchema::integer().min(1).max(65535))
            .require("name");

        registry.register(schema).unwrap();

        let loader = ConfigurationLoader::new(registry);

        let content = r#"
            name = "MyApp"
            port = 8080
        "#;

        let value = loader.load_string(content, "test.hcl").unwrap();
        assert_eq!(value.get("name"), Some(&Value::String("MyApp".to_string())));
        assert_eq!(value.get("port"), Some(&Value::Integer(8080)));
    }
}
