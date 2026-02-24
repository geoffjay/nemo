//! Nemo Configuration Engine - XML parsing and schema validation.
//!
//! This crate provides the configuration system for Nemo applications,
//! including XML parsing, schema validation, and expression resolution.

mod error;
mod loader;
mod location;
mod path;
mod registry;
mod resolver;
mod schema;
mod validator;
mod value;
mod xml_parser;

pub use error::{ConfigError, ErrorCode, ParseError, ResolveError, SchemaError, ValidationError};
pub use loader::ConfigurationLoader;
pub use location::SourceLocation;
pub use path::{ConfigPath, PathParseError, PathSegment};
pub use xml_parser::XmlParser;
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
        <nemo>
            <variable name="app_name" default="MyApp" />
        </nemo>
        "#;

        let value = loader.load_xml_string(content, "test.xml", None).unwrap();
        let vars = value.get("variable").unwrap();
        let app_name = vars.get("app_name").unwrap();
        assert_eq!(
            app_name.get("default"),
            Some(&Value::String("MyApp".to_string()))
        );
    }
}
