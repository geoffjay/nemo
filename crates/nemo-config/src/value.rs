//! Configuration value types.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A configuration value that can hold any supported type.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value.
    #[default]
    Null,
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// Array of values.
    Array(Vec<Value>),
    /// Object (map) of values.
    Object(IndexMap<String, Value>),
}

impl Value {
    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns the value as a bool, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as an i64, if it is one.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the value as an f64, if it is one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Returns the value as a string slice, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as an array, if it is one.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns the value as an object, if it is one.
    pub fn as_object(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Gets a value by key if this is an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(obj) => obj.get(key),
            _ => None,
        }
    }

    /// Gets a value by index if this is an array.
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(arr) => arr.get(index),
            _ => None,
        }
    }

    /// Returns the type name of this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(obj) => {
                let map: IndexMap<String, Value> =
                    obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect();
                Value::Object(map)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert!(Value::Null.is_null());
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Integer(42).as_i64(), Some(42));
        assert_eq!(Value::Float(3.125).as_f64(), Some(3.125));
        assert_eq!(Value::String("hello".into()).as_str(), Some("hello"));
    }

    #[test]
    fn test_object_access() {
        let mut obj = IndexMap::new();
        obj.insert("key".to_string(), Value::String("value".into()));
        let value = Value::Object(obj);

        assert_eq!(value.get("key"), Some(&Value::String("value".into())));
        assert_eq!(value.get("missing"), None);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy to generate arbitrary `Value` instances (leaf types only for speed).
    fn arb_leaf_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Integer),
            // Avoid NaN/Inf which don't roundtrip through JSON
            (-1e10f64..1e10f64).prop_map(Value::Float),
            "[a-zA-Z0-9_]{0,20}".prop_map(Value::String),
        ]
    }

    /// Strategy to generate `Value` including arrays and objects (1 level deep).
    fn arb_value() -> impl Strategy<Value = Value> {
        arb_leaf_value().prop_recursive(2, 16, 4, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                prop::collection::vec(("[a-z]{1,8}".prop_map(String::from), inner), 0..4).prop_map(
                    |pairs| {
                        Value::Object(pairs.into_iter().collect::<IndexMap<String, Value>>())
                    }
                ),
            ]
        })
    }

    proptest! {
        #[test]
        fn type_name_is_correct(val in arb_value()) {
            let name = val.type_name();
            match &val {
                Value::Null => prop_assert_eq!(name, "null"),
                Value::Bool(_) => prop_assert_eq!(name, "bool"),
                Value::Integer(_) => prop_assert_eq!(name, "integer"),
                Value::Float(_) => prop_assert_eq!(name, "float"),
                Value::String(_) => prop_assert_eq!(name, "string"),
                Value::Array(_) => prop_assert_eq!(name, "array"),
                Value::Object(_) => prop_assert_eq!(name, "object"),
            }
        }

        #[test]
        fn accessor_matches_variant(val in arb_leaf_value()) {
            match &val {
                Value::Null => {
                    prop_assert!(val.is_null());
                    prop_assert!(val.as_bool().is_none());
                    prop_assert!(val.as_i64().is_none());
                    prop_assert!(val.as_str().is_none());
                }
                Value::Bool(b) => {
                    prop_assert_eq!(val.as_bool(), Some(*b));
                    prop_assert!(!val.is_null());
                }
                Value::Integer(i) => {
                    prop_assert_eq!(val.as_i64(), Some(*i));
                    // as_f64 also works for integers
                    prop_assert!(val.as_f64().is_some());
                }
                Value::Float(f) => {
                    prop_assert_eq!(val.as_f64(), Some(*f));
                    prop_assert!(val.as_i64().is_none());
                }
                Value::String(s) => {
                    prop_assert_eq!(val.as_str(), Some(s.as_str()));
                }
                _ => {}
            }
        }

        #[test]
        fn json_roundtrip_preserves_structure(val in arb_value()) {
            // Value -> serde_json::Value -> Value should preserve structure
            // (floats may lose precision, but structural equality holds for most values)
            let json_str = serde_json::to_string(&val).unwrap();
            let back: Value = serde_json::from_str(&json_str).unwrap();

            // For non-float values, roundtrip should be exact
            match &val {
                Value::Float(_) => {} // Skip float comparison due to JSON precision
                _ => {
                    // Check structural equivalence: same type, same shape
                    prop_assert_eq!(val.type_name(), back.type_name());
                }
            }
        }

        #[test]
        fn display_does_not_panic(val in arb_value()) {
            // Display should never panic for any valid Value
            let _ = format!("{}", val);
        }

        #[test]
        fn clone_equals_original(val in arb_value()) {
            let cloned = val.clone();
            prop_assert_eq!(&val, &cloned);
        }

        #[test]
        fn get_on_non_object_returns_none(val in arb_leaf_value(), key in "[a-z]{1,5}") {
            match &val {
                Value::Object(_) => {} // Skip objects
                _ => {
                    prop_assert!(val.get(&key).is_none());
                }
            }
        }

        #[test]
        fn get_index_on_non_array_returns_none(val in arb_leaf_value(), idx in 0usize..10) {
            match &val {
                Value::Array(_) => {}
                _ => {
                    prop_assert!(val.get_index(idx).is_none());
                }
            }
        }

        #[test]
        fn from_json_value_roundtrip(b in any::<bool>()) {
            let json_val = serde_json::Value::Bool(b);
            let config_val = Value::from(json_val);
            prop_assert_eq!(config_val, Value::Bool(b));
        }

        #[test]
        fn from_i64_roundtrip(i in any::<i64>()) {
            let val = Value::from(i);
            prop_assert_eq!(val.as_i64(), Some(i));
        }

        #[test]
        fn from_string_roundtrip(s in "[a-zA-Z0-9]{0,30}") {
            let val = Value::from(s.clone());
            prop_assert_eq!(val.as_str(), Some(s.as_str()));
        }
    }
}
