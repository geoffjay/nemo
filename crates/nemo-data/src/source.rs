//! Data source trait and related types.

use crate::error::DataSourceError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Unique identifier for a data source.
pub type SourceId = String;

/// Status of a data source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceStatus {
    /// Not connected.
    #[default]
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connected and receiving data.
    Connected,
    /// In error state.
    Error(String),
}

/// Type of data update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateType {
    /// Complete data replacement.
    Full,
    /// Partial update (patch/delta).
    Partial,
    /// Append to collection.
    Append,
    /// Delete from collection.
    Delete,
}

/// A data update from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataUpdate {
    /// ID of the source that produced this update.
    pub source_id: SourceId,
    /// When the update was produced.
    pub timestamp: DateTime<Utc>,
    /// The data payload.
    pub data: Value,
    /// Type of update.
    pub update_type: UpdateType,
}

impl DataUpdate {
    /// Creates a new full data update.
    pub fn full(source_id: impl Into<String>, data: Value) -> Self {
        Self {
            source_id: source_id.into(),
            timestamp: Utc::now(),
            data,
            update_type: UpdateType::Full,
        }
    }

    /// Creates a new partial data update.
    pub fn partial(source_id: impl Into<String>, data: Value) -> Self {
        Self {
            source_id: source_id.into(),
            timestamp: Utc::now(),
            data,
            update_type: UpdateType::Partial,
        }
    }

    /// Creates a new append update.
    pub fn append(source_id: impl Into<String>, data: Value) -> Self {
        Self {
            source_id: source_id.into(),
            timestamp: Utc::now(),
            data,
            update_type: UpdateType::Append,
        }
    }
}

/// Schema describing the data a source produces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataSchema {
    /// Name of the schema.
    pub name: String,
    /// Description of the data.
    pub description: String,
    /// Expected value type.
    pub value_type: SchemaType,
}

/// Type information for schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaType {
    /// Any type.
    #[default]
    Any,
    /// Null value.
    Null,
    /// Boolean value.
    Boolean,
    /// Integer value.
    Integer,
    /// Floating point value.
    Float,
    /// String value.
    String,
    /// Array of values.
    Array(Box<SchemaType>),
    /// Object with string keys.
    Object,
}

/// Abstract interface for all data sources.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Unique identifier for this source.
    fn id(&self) -> &str;

    /// Schema of data this source produces.
    fn schema(&self) -> &DataSchema;

    /// Start collecting data.
    async fn start(&mut self) -> Result<(), DataSourceError>;

    /// Stop collecting data.
    async fn stop(&mut self) -> Result<(), DataSourceError>;

    /// Subscribe to data updates.
    fn subscribe(&self) -> broadcast::Receiver<DataUpdate>;

    /// Current connection status.
    fn status(&self) -> SourceStatus;

    /// Manual refresh (for polling sources).
    async fn refresh(&mut self) -> Result<(), DataSourceError>;
}

/// Configuration common to all data sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Unique ID for this source instance.
    pub id: String,
    /// Whether to start immediately.
    pub auto_start: bool,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            auto_start: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_update_full() {
        let update = DataUpdate::full("test", Value::String("data".into()));
        assert_eq!(update.source_id, "test");
        assert_eq!(update.update_type, UpdateType::Full);
    }

    #[test]
    fn test_source_status_default() {
        let status = SourceStatus::default();
        assert_eq!(status, SourceStatus::Disconnected);
    }
}
