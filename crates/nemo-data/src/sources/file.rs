//! File watcher data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

/// File format.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    /// JSON format.
    #[default]
    Json,
    /// YAML format.
    Yaml,
    /// TOML format.
    Toml,
    /// CSV format.
    Csv,
    /// Line-delimited text.
    Lines,
    /// Raw text.
    Raw,
}

/// Configuration for a file data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// Path to the file.
    pub path: PathBuf,
    /// File format.
    #[serde(default)]
    pub format: FileFormat,
    /// Whether to watch for changes.
    #[serde(default)]
    pub watch: bool,
    /// Debounce duration for file changes.
    #[serde(default = "default_debounce")]
    pub debounce: Duration,
}

fn default_debounce() -> Duration {
    Duration::from_millis(100)
}

impl Default for FileSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            path: PathBuf::new(),
            format: FileFormat::default(),
            watch: false,
            debounce: default_debounce(),
        }
    }
}

/// File watcher data source.
pub struct FileSource {
    config: FileSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    _watcher: Option<RecommendedWatcher>,
    task: Option<JoinHandle<()>>,
}

impl FileSource {
    /// Creates a new file source.
    pub fn new(config: FileSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("File data from {}", config.path.display()),
            value_type: SchemaType::Any,
        };

        Self {
            config,
            status: Arc::new(RwLock::new(SourceStatus::Disconnected)),
            sender,
            schema,
            _watcher: None,
            task: None,
        }
    }

    /// Reads and parses the file.
    async fn read_file(&self) -> Result<Value, DataSourceError> {
        let content = tokio::fs::read_to_string(&self.config.path).await?;

        let value = match self.config.format {
            FileFormat::Json => {
                let json: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| DataSourceError::Parse(e.to_string()))?;
                Value::from(json)
            }
            FileFormat::Raw => Value::String(content),
            FileFormat::Lines => {
                let lines: Vec<Value> = content
                    .lines()
                    .map(|l| Value::String(l.to_string()))
                    .collect();
                Value::Array(lines)
            }
            FileFormat::Yaml | FileFormat::Toml | FileFormat::Csv => {
                // Simplified - just return as string for now
                Value::String(content)
            }
        };

        Ok(value)
    }
}

#[async_trait]
impl DataSource for FileSource {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn schema(&self) -> &DataSchema {
        &self.schema
    }

    async fn start(&mut self) -> Result<(), DataSourceError> {
        if self.task.is_some() {
            return Err(DataSourceError::AlreadyRunning);
        }

        *self.status.write().await = SourceStatus::Connecting;

        // Initial read
        match self.read_file().await {
            Ok(data) => {
                *self.status.write().await = SourceStatus::Connected;
                let update = DataUpdate::full(&self.config.id, data);
                let _ = self.sender.send(update);
            }
            Err(e) => {
                *self.status.write().await = SourceStatus::Error(e.to_string());
                return Err(e);
            }
        }

        // Set up file watcher if enabled
        if self.config.watch {
            let (tx, mut rx) = mpsc::channel::<Result<Event, notify::Error>>(100);
            let path = self.config.path.clone();
            let config = self.config.clone();
            let status = self.status.clone();
            let sender = self.sender.clone();

            // Create watcher
            let mut watcher = notify::recommended_watcher(move |res| {
                let _ = tx.blocking_send(res);
            })
            .map_err(|e| DataSourceError::Io(std::io::Error::other(e)))?;

            watcher
                .watch(&path, RecursiveMode::NonRecursive)
                .map_err(|e| {
                    DataSourceError::Io(std::io::Error::other(e))
                })?;

            self._watcher = Some(watcher);

            // Start task to handle file events
            let task = tokio::spawn(async move {
                let debounce = config.debounce;
                let mut last_event = std::time::Instant::now();

                while let Some(event) = rx.recv().await {
                    if event.is_ok() {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_event) >= debounce {
                            last_event = now;

                            // Read and send update
                            match tokio::fs::read_to_string(&config.path).await {
                                Ok(content) => {
                                    let value = match config.format {
                                        FileFormat::Json => {
                                            if let Ok(json) =
                                                serde_json::from_str::<serde_json::Value>(&content)
                                            {
                                                Value::from(json)
                                            } else {
                                                continue;
                                            }
                                        }
                                        FileFormat::Raw => Value::String(content),
                                        FileFormat::Lines => {
                                            let lines: Vec<Value> = content
                                                .lines()
                                                .map(|l| Value::String(l.to_string()))
                                                .collect();
                                            Value::Array(lines)
                                        }
                                        _ => Value::String(content),
                                    };

                                    *status.write().await = SourceStatus::Connected;
                                    let update = DataUpdate::full(&config.id, value);
                                    let _ = sender.send(update);
                                }
                                Err(e) => {
                                    *status.write().await = SourceStatus::Error(e.to_string());
                                }
                            }
                        }
                    }
                }
            });

            self.task = Some(task);
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DataSourceError> {
        self._watcher = None;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        *self.status.write().await = SourceStatus::Disconnected;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<DataUpdate> {
        self.sender.subscribe()
    }

    fn status(&self) -> SourceStatus {
        self.status
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(SourceStatus::Disconnected)
    }

    async fn refresh(&mut self) -> Result<(), DataSourceError> {
        let data = self.read_file().await?;
        let update = DataUpdate::full(&self.config.id, data);
        let _ = self.sender.send(update);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_config_default() {
        let config = FileSourceConfig::default();
        assert_eq!(config.format, FileFormat::Json);
        assert!(!config.watch);
    }

    #[test]
    fn test_file_source_creation() {
        let config = FileSourceConfig {
            id: "test".into(),
            path: PathBuf::from("/tmp/test.json"),
            ..Default::default()
        };
        let source = FileSource::new(config);
        assert_eq!(source.id(), "test");
    }
}
