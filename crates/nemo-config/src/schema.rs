//! Configuration schema definitions.

use crate::Value;
use indexmap::IndexMap;
use std::fmt;

/// Schema for a configuration object.
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    /// Name of this schema.
    pub name: String,
    /// Description of the schema.
    pub description: Option<String>,
    /// Properties defined in this schema.
    pub properties: IndexMap<String, PropertySchema>,
    /// Required property names.
    pub required: Vec<String>,
    /// Whether additional properties are allowed.
    pub additional_properties: bool,
}

impl ConfigSchema {
    /// Creates a new schema with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        ConfigSchema {
            name: name.into(),
            description: None,
            properties: IndexMap::new(),
            required: Vec::new(),
            additional_properties: true,
        }
    }

    /// Sets the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a property to the schema.
    pub fn property(mut self, name: impl Into<String>, schema: PropertySchema) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    /// Marks a property as required.
    pub fn require(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }

    /// Disallows additional properties.
    pub fn strict(mut self) -> Self {
        self.additional_properties = false;
        self
    }
}

/// Schema for a configuration property.
#[derive(Debug, Clone)]
pub struct PropertySchema {
    /// The expected type.
    pub value_type: ValueType,
    /// Description of this property.
    pub description: Option<String>,
    /// Default value if not specified.
    pub default: Option<Value>,
    /// Validation rules.
    pub rules: Vec<ValidationRule>,
    /// For arrays, the schema of items.
    pub items: Option<Box<PropertySchema>>,
    /// For objects, the nested schema.
    pub nested: Option<Box<ConfigSchema>>,
}

impl PropertySchema {
    /// Creates a string property schema.
    pub fn string() -> Self {
        PropertySchema {
            value_type: ValueType::String,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: None,
        }
    }

    /// Creates an integer property schema.
    pub fn integer() -> Self {
        PropertySchema {
            value_type: ValueType::Integer,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: None,
        }
    }

    /// Creates a float property schema.
    pub fn float() -> Self {
        PropertySchema {
            value_type: ValueType::Float,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: None,
        }
    }

    /// Creates a boolean property schema.
    pub fn boolean() -> Self {
        PropertySchema {
            value_type: ValueType::Boolean,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: None,
        }
    }

    /// Creates an array property schema.
    pub fn array(items: PropertySchema) -> Self {
        PropertySchema {
            value_type: ValueType::Array,
            description: None,
            default: None,
            rules: Vec::new(),
            items: Some(Box::new(items)),
            nested: None,
        }
    }

    /// Creates an object property schema.
    pub fn object(schema: ConfigSchema) -> Self {
        PropertySchema {
            value_type: ValueType::Object,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: Some(Box::new(schema)),
        }
    }

    /// Creates an any-type property schema.
    pub fn any() -> Self {
        PropertySchema {
            value_type: ValueType::Any,
            description: None,
            default: None,
            rules: Vec::new(),
            items: None,
            nested: None,
        }
    }

    /// Sets the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets a default value.
    pub fn with_default(mut self, default: impl Into<Value>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Adds a validation rule.
    pub fn rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Adds a minimum value rule.
    pub fn min(self, min: i64) -> Self {
        self.rule(ValidationRule::Min(min))
    }

    /// Adds a maximum value rule.
    pub fn max(self, max: i64) -> Self {
        self.rule(ValidationRule::Max(max))
    }

    /// Adds a pattern rule for strings.
    pub fn pattern(self, pattern: impl Into<String>) -> Self {
        self.rule(ValidationRule::Pattern(pattern.into()))
    }

    /// Adds an enum constraint.
    pub fn one_of(self, values: Vec<Value>) -> Self {
        self.rule(ValidationRule::OneOf(values))
    }
}

/// The type of a configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Any,
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::String => write!(f, "string"),
            ValueType::Integer => write!(f, "integer"),
            ValueType::Float => write!(f, "float"),
            ValueType::Boolean => write!(f, "boolean"),
            ValueType::Array => write!(f, "array"),
            ValueType::Object => write!(f, "object"),
            ValueType::Any => write!(f, "any"),
        }
    }
}

/// A validation rule for a property.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Minimum value (for numbers).
    Min(i64),
    /// Maximum value (for numbers).
    Max(i64),
    /// Minimum length (for strings/arrays).
    MinLength(usize),
    /// Maximum length (for strings/arrays).
    MaxLength(usize),
    /// Pattern match (for strings).
    Pattern(String),
    /// Value must be one of these.
    OneOf(Vec<Value>),
    /// Custom validation.
    Custom { name: String, message: String },
}

impl fmt::Display for ValidationRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationRule::Min(v) => write!(f, "min({})", v),
            ValidationRule::Max(v) => write!(f, "max({})", v),
            ValidationRule::MinLength(v) => write!(f, "minLength({})", v),
            ValidationRule::MaxLength(v) => write!(f, "maxLength({})", v),
            ValidationRule::Pattern(p) => write!(f, "pattern({})", p),
            ValidationRule::OneOf(v) => write!(f, "oneOf({:?})", v),
            ValidationRule::Custom { name, .. } => write!(f, "custom({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_builder() {
        let schema = ConfigSchema::new("application")
            .description("Application configuration")
            .property("name", PropertySchema::string())
            .property("version", PropertySchema::string().with_default("1.0.0"))
            .property("port", PropertySchema::integer().min(1).max(65535))
            .require("name")
            .strict();

        assert_eq!(schema.name, "application");
        assert!(schema.properties.contains_key("name"));
        assert!(schema.required.contains(&"name".to_string()));
        assert!(!schema.additional_properties);
    }
}
