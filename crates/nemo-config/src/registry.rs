//! Schema registry for storing and retrieving configuration schemas.

use crate::error::SchemaError;
use crate::schema::ConfigSchema;
use std::collections::HashMap;
use std::sync::RwLock;

/// Registry for configuration schemas.
pub struct SchemaRegistry {
    schemas: RwLock<HashMap<String, ConfigSchema>>,
}

impl SchemaRegistry {
    /// Creates a new empty schema registry.
    pub fn new() -> Self {
        SchemaRegistry {
            schemas: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a schema.
    pub fn register(&self, schema: ConfigSchema) -> Result<(), SchemaError> {
        let mut schemas = self.schemas.write().map_err(|_| SchemaError::LockError)?;

        if schemas.contains_key(&schema.name) {
            return Err(SchemaError::AlreadyRegistered {
                name: schema.name.clone(),
            });
        }

        schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// Gets a schema by name.
    pub fn get(&self, name: &str) -> Option<ConfigSchema> {
        self.schemas
            .read()
            .ok()
            .and_then(|schemas| schemas.get(name).cloned())
    }

    /// Checks if a schema is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.schemas
            .read()
            .ok()
            .map(|schemas| schemas.contains_key(name))
            .unwrap_or(false)
    }

    /// Returns all registered schema names.
    pub fn names(&self) -> Vec<String> {
        self.schemas
            .read()
            .ok()
            .map(|schemas| schemas.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Unregisters a schema.
    pub fn unregister(&self, name: &str) -> Option<ConfigSchema> {
        self.schemas
            .write()
            .ok()
            .and_then(|mut schemas| schemas.remove(name))
    }

    /// Clears all registered schemas.
    pub fn clear(&self) {
        if let Ok(mut schemas) = self.schemas.write() {
            schemas.clear();
        }
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::PropertySchema;

    #[test]
    fn test_register_and_get() {
        let registry = SchemaRegistry::new();

        let schema = ConfigSchema::new("test").property("name", PropertySchema::string());

        registry.register(schema).unwrap();

        assert!(registry.contains("test"));
        let retrieved = registry.get("test").unwrap();
        assert_eq!(retrieved.name, "test");
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = SchemaRegistry::new();

        let schema1 = ConfigSchema::new("test");
        let schema2 = ConfigSchema::new("test");

        registry.register(schema1).unwrap();
        let result = registry.register(schema2);

        assert!(matches!(result, Err(SchemaError::AlreadyRegistered { .. })));
    }

    #[test]
    fn test_unregister() {
        let registry = SchemaRegistry::new();

        registry.register(ConfigSchema::new("test")).unwrap();
        assert!(registry.contains("test"));

        registry.unregister("test");
        assert!(!registry.contains("test"));
    }
}
