//! HTTP polling data source.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

/// HTTP method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Get
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial backoff duration.
    pub initial_backoff: Duration,
    /// Maximum backoff duration.
    pub max_backoff: Duration,
    /// Backoff multiplier.
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// Configuration for an HTTP data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// URL to fetch.
    pub url: String,
    /// HTTP method.
    #[serde(default)]
    pub method: HttpMethod,
    /// Request headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (for POST/PUT/PATCH).
    pub body: Option<Value>,
    /// Polling interval.
    #[serde(default)]
    pub interval: Option<Duration>,
    /// Request timeout.
    #[serde(default = "default_timeout")]
    pub timeout: Duration,
    /// Retry configuration.
    #[serde(default)]
    pub retry: RetryConfig,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

impl Default for HttpSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: String::new(),
            method: HttpMethod::Get,
            headers: HashMap::new(),
            body: None,
            interval: None,
            timeout: default_timeout(),
            retry: RetryConfig::default(),
        }
    }
}

/// HTTP polling data source.
pub struct HttpSource {
    config: HttpSourceConfig,
    client: reqwest::Client,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
}

impl HttpSource {
    /// Creates a new HTTP source.
    pub fn new(config: HttpSourceConfig) -> Self {
        let (sender, _) = broadcast::channel(100);

        let schema = DataSchema {
            name: config.id.clone(),
            description: format!("HTTP data from {}", config.url),
            value_type: SchemaType::Any,
        };

        Self {
            config,
            client: reqwest::Client::new(),
            status: Arc::new(RwLock::new(SourceStatus::Disconnected)),
            sender,
            schema,
            task: None,
        }
    }

    /// Performs a single fetch.
    async fn fetch(&self) -> Result<Value, DataSourceError> {
        let mut request = match self.config.method {
            HttpMethod::Get => self.client.get(&self.config.url),
            HttpMethod::Post => self.client.post(&self.config.url),
            HttpMethod::Put => self.client.put(&self.config.url),
            HttpMethod::Patch => self.client.patch(&self.config.url),
            HttpMethod::Delete => self.client.delete(&self.config.url),
        };

        // Add headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body) = &self.config.body {
            request = request.json(body);
        }

        // Set timeout
        request = request.timeout(self.config.timeout);

        let response = request
            .send()
            .await
            .map_err(|e| DataSourceError::Request(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DataSourceError::Request(format!(
                "HTTP {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DataSourceError::Parse(e.to_string()))?;

        Ok(Value::from(json))
    }
}

#[async_trait]
impl DataSource for HttpSource {
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

        // Do an initial fetch
        match self.fetch().await {
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

        // If interval is set, start polling
        if let Some(interval) = self.config.interval {
            let config = self.config.clone();
            let client = self.client.clone();
            let status = self.status.clone();
            let sender = self.sender.clone();

            let task = tokio::spawn(async move {
                let mut interval_timer = tokio::time::interval(interval);
                interval_timer.tick().await; // Skip first tick

                loop {
                    interval_timer.tick().await;

                    let mut request = match config.method {
                        HttpMethod::Get => client.get(&config.url),
                        HttpMethod::Post => client.post(&config.url),
                        HttpMethod::Put => client.put(&config.url),
                        HttpMethod::Patch => client.patch(&config.url),
                        HttpMethod::Delete => client.delete(&config.url),
                    };

                    for (key, value) in &config.headers {
                        request = request.header(key, value);
                    }

                    if let Some(body) = &config.body {
                        request = request.json(body);
                    }

                    request = request.timeout(config.timeout);

                    match request.send().await {
                        Ok(response) if response.status().is_success() => {
                            if let Ok(json) = response.json::<serde_json::Value>().await {
                                *status.write().await = SourceStatus::Connected;
                                let update = DataUpdate::full(&config.id, Value::from(json));
                                let _ = sender.send(update);
                            }
                        }
                        Ok(response) => {
                            let err_msg = format!("HTTP {}", response.status());
                            *status.write().await = SourceStatus::Error(err_msg);
                        }
                        Err(e) => {
                            *status.write().await = SourceStatus::Error(e.to_string());
                        }
                    }
                }
            });

            self.task = Some(task);
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DataSourceError> {
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
        // Use try_read to avoid blocking
        self.status
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(SourceStatus::Disconnected)
    }

    async fn refresh(&mut self) -> Result<(), DataSourceError> {
        let data = self.fetch().await?;
        let update = DataUpdate::full(&self.config.id, data);
        let _ = self.sender.send(update);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpSourceConfig::default();
        assert_eq!(config.method, HttpMethod::Get);
        assert!(config.headers.is_empty());
    }

    #[test]
    fn test_http_source_creation() {
        let config = HttpSourceConfig {
            id: "test".into(),
            url: "https://example.com/api".into(),
            ..Default::default()
        };
        let source = HttpSource::new(config);
        assert_eq!(source.id(), "test");
        assert_eq!(source.status(), SourceStatus::Disconnected);
    }
}
