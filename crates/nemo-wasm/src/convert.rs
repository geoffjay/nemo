//! Bidirectional conversion between `nemo_plugin_api::PluginValue` and WIT-generated types.
//!
//! Since WIT does not support recursive types, complex values (arrays and objects)
//! are serialized to/from JSON strings via the `json-val` variant.

use crate::nemo::plugin::types::{LogLevel as WitLogLevel, PluginValue as WitPluginValue};
use nemo_plugin_api::{LogLevel, PluginValue};

/// Converts a WIT `PluginValue` into a `nemo_plugin_api::PluginValue`.
pub fn from_wit(value: &WitPluginValue) -> PluginValue {
    match value {
        WitPluginValue::Null => PluginValue::Null,
        WitPluginValue::BoolVal(b) => PluginValue::Bool(*b),
        WitPluginValue::IntegerVal(i) => PluginValue::Integer(*i),
        WitPluginValue::FloatVal(f) => PluginValue::Float(*f),
        WitPluginValue::StringVal(s) => PluginValue::String(s.clone()),
        WitPluginValue::JsonVal(json) => serde_json::from_str(json).unwrap_or(PluginValue::Null),
    }
}

/// Converts a `nemo_plugin_api::PluginValue` into a WIT `PluginValue`.
pub fn to_wit(value: &PluginValue) -> WitPluginValue {
    match value {
        PluginValue::Null => WitPluginValue::Null,
        PluginValue::Bool(b) => WitPluginValue::BoolVal(*b),
        PluginValue::Integer(i) => WitPluginValue::IntegerVal(*i),
        PluginValue::Float(f) => WitPluginValue::FloatVal(*f),
        PluginValue::String(s) => WitPluginValue::StringVal(s.clone()),
        PluginValue::Array(_) | PluginValue::Object(_) => {
            let json = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
            WitPluginValue::JsonVal(json)
        }
    }
}

/// Converts a WIT `LogLevel` into a `nemo_plugin_api::LogLevel`.
pub fn log_level_from_wit(level: WitLogLevel) -> LogLevel {
    match level {
        WitLogLevel::Debug => LogLevel::Debug,
        WitLogLevel::Info => LogLevel::Info,
        WitLogLevel::Warn => LogLevel::Warn,
        WitLogLevel::Error => LogLevel::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_round_trip_null() {
        let original = PluginValue::Null;
        let wit = to_wit(&original);
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_bool() {
        let original = PluginValue::Bool(true);
        let wit = to_wit(&original);
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_integer() {
        let original = PluginValue::Integer(42);
        let wit = to_wit(&original);
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_float() {
        let original = PluginValue::Float(3.125);
        let wit = to_wit(&original);
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_string() {
        let original = PluginValue::String("hello".into());
        let wit = to_wit(&original);
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_array() {
        let original = PluginValue::Array(vec![
            PluginValue::Integer(1),
            PluginValue::String("two".into()),
        ]);
        let wit = to_wit(&original);
        assert!(matches!(wit, WitPluginValue::JsonVal(_)));
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_round_trip_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), PluginValue::Bool(false));
        let original = PluginValue::Object(map);
        let wit = to_wit(&original);
        assert!(matches!(wit, WitPluginValue::JsonVal(_)));
        let back = from_wit(&wit);
        assert_eq!(back, original);
    }

    #[test]
    fn test_log_level_conversion() {
        assert!(matches!(
            log_level_from_wit(WitLogLevel::Debug),
            LogLevel::Debug
        ));
        assert!(matches!(
            log_level_from_wit(WitLogLevel::Info),
            LogLevel::Info
        ));
        assert!(matches!(
            log_level_from_wit(WitLogLevel::Warn),
            LogLevel::Warn
        ));
        assert!(matches!(
            log_level_from_wit(WitLogLevel::Error),
            LogLevel::Error
        ));
    }
}
