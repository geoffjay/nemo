//! Value helpers for creating PluginValue instances

use indexmap::IndexMap;
use nemo_plugin_api::PluginValue;

/// Helper trait for types that can be converted to PluginValue
pub trait IntoPluginValue {
    fn into_plugin_value(self) -> PluginValue;
}

impl IntoPluginValue for PluginValue {
    fn into_plugin_value(self) -> PluginValue {
        self
    }
}

impl IntoPluginValue for &str {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::String(self.to_string())
    }
}

impl IntoPluginValue for String {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::String(self)
    }
}

impl IntoPluginValue for i64 {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::Integer(self)
    }
}

impl IntoPluginValue for i32 {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::Integer(self as i64)
    }
}

impl IntoPluginValue for f64 {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::Float(self)
    }
}

impl IntoPluginValue for bool {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::Bool(self)
    }
}

impl<T: IntoPluginValue> IntoPluginValue for Vec<T> {
    fn into_plugin_value(self) -> PluginValue {
        PluginValue::Array(self.into_iter().map(|v| v.into_plugin_value()).collect())
    }
}

/// Creates a PluginValue::String
pub fn str_value(s: impl Into<String>) -> PluginValue {
    PluginValue::String(s.into())
}

/// Creates a PluginValue::Integer
pub fn int_value(i: impl Into<i64>) -> PluginValue {
    PluginValue::Integer(i.into())
}

/// Creates a PluginValue::Float
pub fn float_value(f: impl Into<f64>) -> PluginValue {
    PluginValue::Float(f.into())
}

/// Creates a PluginValue::Bool
pub fn bool_value(b: bool) -> PluginValue {
    PluginValue::Bool(b)
}

/// Creates a PluginValue::Array
pub fn array_value<T: IntoPluginValue>(items: Vec<T>) -> PluginValue {
    PluginValue::Array(items.into_iter().map(|v| v.into_plugin_value()).collect())
}

/// Creates a PluginValue::Object from key-value pairs
pub fn object_value(pairs: &[(&str, PluginValue)]) -> PluginValue {
    let mut map = IndexMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v.clone());
    }
    PluginValue::Object(map)
}
