//! Data repository - storage with change notification.

use crate::error::RepositoryError;
use chrono::{DateTime, Utc};
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tokio::sync::broadcast;

/// A path to data in the repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataPath {
    segments: Vec<PathSegment>,
}

/// A segment of a data path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathSegment {
    /// Property access.
    Property(String),
    /// Array index.
    Index(usize),
    /// Wildcard (matches any single segment).
    Wildcard,
}

impl DataPath {
    /// Creates a new data path from a string.
    pub fn parse(s: &str) -> Result<Self, RepositoryError> {
        let mut segments = Vec::new();

        for part in s.split('.') {
            if part.is_empty() {
                continue;
            }

            if part == "*" {
                segments.push(PathSegment::Wildcard);
            } else if let Ok(idx) = part.parse::<usize>() {
                segments.push(PathSegment::Index(idx));
            } else if part.contains('[') && part.contains(']') {
                // Handle array notation like "items[0]"
                let bracket_start = part.find('[').unwrap();
                let bracket_end = part.find(']').unwrap();

                if bracket_start > 0 {
                    segments.push(PathSegment::Property(part[..bracket_start].to_string()));
                }

                let idx_str = &part[bracket_start + 1..bracket_end];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
            } else {
                segments.push(PathSegment::Property(part.to_string()));
            }
        }

        Ok(Self { segments })
    }

    /// Creates a path from a source ID.
    pub fn from_source(source_id: &str) -> Self {
        Self {
            segments: vec![
                PathSegment::Property("data".to_string()),
                PathSegment::Property(source_id.to_string()),
            ],
        }
    }

    /// Gets a value at this path from a root value.
    pub fn get<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        let mut current = root;

        for segment in &self.segments {
            match segment {
                PathSegment::Property(key) => {
                    if let Value::Object(obj) = current {
                        current = obj.get(key)?;
                    } else {
                        return None;
                    }
                }
                PathSegment::Index(idx) => {
                    if let Value::Array(arr) = current {
                        current = arr.get(*idx)?;
                    } else {
                        return None;
                    }
                }
                PathSegment::Wildcard => {
                    // Wildcard not supported in get
                    return None;
                }
            }
        }

        Some(current)
    }

    /// Sets a value at this path in a root value.
    pub fn set(&self, root: &mut Value, value: Value) -> Result<(), RepositoryError> {
        if self.segments.is_empty() {
            *root = value;
            return Ok(());
        }

        let mut current = root;

        for (i, segment) in self.segments.iter().enumerate() {
            let is_last = i == self.segments.len() - 1;

            match segment {
                PathSegment::Property(key) => {
                    if is_last {
                        if let Value::Object(obj) = current {
                            obj.insert(key.clone(), value);
                            return Ok(());
                        } else {
                            return Err(RepositoryError::TypeMismatch {
                                path: self.to_string(),
                                expected: "object".to_string(),
                                actual: format!("{:?}", current),
                            });
                        }
                    } else {
                        if let Value::Object(obj) = current {
                            if !obj.contains_key(key) {
                                // Create intermediate object
                                obj.insert(key.clone(), Value::Object(indexmap::IndexMap::new()));
                            }
                            current = obj.get_mut(key).unwrap();
                        } else {
                            return Err(RepositoryError::TypeMismatch {
                                path: self.to_string(),
                                expected: "object".to_string(),
                                actual: format!("{:?}", current),
                            });
                        }
                    }
                }
                PathSegment::Index(idx) => {
                    if let Value::Array(arr) = current {
                        if *idx >= arr.len() {
                            return Err(RepositoryError::InvalidPath(format!(
                                "Index {} out of bounds (length {})",
                                idx,
                                arr.len()
                            )));
                        }
                        if is_last {
                            arr[*idx] = value;
                            return Ok(());
                        } else {
                            current = &mut arr[*idx];
                        }
                    } else {
                        return Err(RepositoryError::TypeMismatch {
                            path: self.to_string(),
                            expected: "array".to_string(),
                            actual: format!("{:?}", current),
                        });
                    }
                }
                PathSegment::Wildcard => {
                    return Err(RepositoryError::InvalidPath(
                        "Cannot set with wildcard in path".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Checks if this path matches another path (for subscriptions).
    pub fn matches(&self, other: &DataPath) -> bool {
        if self.segments.len() != other.segments.len() {
            return false;
        }

        for (a, b) in self.segments.iter().zip(other.segments.iter()) {
            match (a, b) {
                (PathSegment::Wildcard, _) | (_, PathSegment::Wildcard) => continue,
                (PathSegment::Property(pa), PathSegment::Property(pb)) if pa == pb => continue,
                (PathSegment::Index(ia), PathSegment::Index(ib)) if ia == ib => continue,
                _ => return false,
            }
        }

        true
    }

    /// Returns the string representation.
    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                PathSegment::Property(p) => p.clone(),
                PathSegment::Index(i) => i.to_string(),
                PathSegment::Wildcard => "*".to_string(),
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl std::fmt::Display for DataPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// A change in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryChange {
    /// Path that changed.
    pub path: DataPath,
    /// Previous value (if any).
    pub old_value: Option<Value>,
    /// New value (if any).
    pub new_value: Option<Value>,
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
}

/// Trait for data stores.
pub trait DataStore: Send + Sync {
    /// Gets a value by key.
    fn get(&self, key: &str) -> Option<Value>;
    /// Sets a value by key.
    fn set(&mut self, key: &str, value: Value);
    /// Deletes a value by key.
    fn delete(&mut self, key: &str);
    /// Returns all keys.
    fn keys(&self) -> Vec<String>;
    /// Clears all data.
    fn clear(&mut self);
}

/// In-memory data store.
#[derive(Debug, Default)]
pub struct MemoryStore {
    data: HashMap<String, Value>,
}

impl MemoryStore {
    /// Creates a new memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl DataStore for MemoryStore {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }

    fn delete(&mut self, key: &str) {
        self.data.remove(key);
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn clear(&mut self) {
        self.data.clear();
    }
}

/// Central data repository.
pub struct DataRepository {
    /// Root value containing all data.
    root: RwLock<Value>,
    /// Named stores.
    stores: RwLock<HashMap<String, Box<dyn DataStore>>>,
    /// Change notification sender.
    change_sender: broadcast::Sender<RepositoryChange>,
}

impl DataRepository {
    /// Creates a new data repository.
    pub fn new() -> Self {
        let (change_sender, _) = broadcast::channel(100);

        let mut root = indexmap::IndexMap::new();
        root.insert("data".to_string(), Value::Object(indexmap::IndexMap::new()));
        root.insert(
            "state".to_string(),
            Value::Object(indexmap::IndexMap::new()),
        );
        root.insert("var".to_string(), Value::Object(indexmap::IndexMap::new()));

        Self {
            root: RwLock::new(Value::Object(root)),
            stores: RwLock::new(HashMap::new()),
            change_sender,
        }
    }

    /// Registers a named data store.
    pub fn register_store(&self, name: &str, store: Box<dyn DataStore>) {
        if let Ok(mut stores) = self.stores.write() {
            stores.insert(name.to_string(), store);
        }
    }

    /// Gets a value by path.
    pub fn get(&self, path: &DataPath) -> Option<Value> {
        self.root
            .read()
            .ok()
            .and_then(|root| path.get(&root).cloned())
    }

    /// Sets a value at a path.
    pub fn set(&self, path: &DataPath, value: Value) -> Result<(), RepositoryError> {
        let old_value = self.get(path);

        {
            let mut root = self.root.write().map_err(|_| RepositoryError::LockError)?;
            path.set(&mut root, value.clone())?;
        }

        let change = RepositoryChange {
            path: path.clone(),
            old_value,
            new_value: Some(value),
            timestamp: Utc::now(),
        };

        let _ = self.change_sender.send(change);
        Ok(())
    }

    /// Deletes a value at a path.
    pub fn delete(&self, path: &DataPath) -> Result<(), RepositoryError> {
        let old_value = self.get(path);

        {
            let mut root = self.root.write().map_err(|_| RepositoryError::LockError)?;
            path.set(&mut root, Value::Null)?;
        }

        let change = RepositoryChange {
            path: path.clone(),
            old_value,
            new_value: None,
            timestamp: Utc::now(),
        };

        let _ = self.change_sender.send(change);
        Ok(())
    }

    /// Subscribes to all changes.
    pub fn subscribe(&self) -> broadcast::Receiver<RepositoryChange> {
        self.change_sender.subscribe()
    }

    /// Updates data from a source.
    pub fn update_from_source(&self, source_id: &str, data: Value) -> Result<(), RepositoryError> {
        let path = DataPath::from_source(source_id);
        self.set(&path, data)
    }
}

impl Default for DataRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_path_parse() {
        let path = DataPath::parse("data.users.0.name").unwrap();
        assert_eq!(path.segments.len(), 4);
    }

    #[test]
    fn test_data_path_get() {
        let mut users = indexmap::IndexMap::new();
        let mut user = indexmap::IndexMap::new();
        user.insert("name".to_string(), Value::String("Alice".into()));
        users.insert("user".to_string(), Value::Object(user));

        let root = Value::Object(users);
        let path = DataPath::parse("user.name").unwrap();

        assert_eq!(path.get(&root), Some(&Value::String("Alice".into())));
    }

    #[test]
    fn test_data_path_set() {
        let mut root = Value::Object(indexmap::IndexMap::new());
        let path = DataPath::parse("user.name").unwrap();

        path.set(&mut root, Value::String("Bob".into())).unwrap();

        assert_eq!(path.get(&root), Some(&Value::String("Bob".into())));
    }

    #[test]
    fn test_memory_store() {
        let mut store = MemoryStore::new();
        store.set("key", Value::Integer(42));
        assert_eq!(store.get("key"), Some(Value::Integer(42)));
        store.delete("key");
        assert_eq!(store.get("key"), None);
    }

    #[test]
    fn test_repository_get_set() {
        let repo = DataRepository::new();
        let path = DataPath::parse("data.test.value").unwrap();

        repo.set(&path, Value::Integer(100)).unwrap();
        assert_eq!(repo.get(&path), Some(Value::Integer(100)));
    }
}
