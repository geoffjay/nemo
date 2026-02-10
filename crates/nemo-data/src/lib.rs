//! Nemo Data Flow Engine - Data collection, transformation, and binding.
//!
//! This crate provides the data flow system for Nemo applications, including:
//! - Data sources for collecting data from various inputs (HTTP, WebSocket, files, timers)
//! - Transformation pipelines for processing data
//! - A central repository for storing and observing data
//! - An action system for triggering operations based on data conditions
//! - A binding system for connecting data to UI components

pub mod action;
pub mod binding;
pub mod error;
pub mod repository;
pub mod source;
pub mod sources;
pub mod transform;

pub use action::{Action, ActionContext, ActionId, ActionSystem, ActionTrigger, TriggerCondition};
pub use binding::{
    Binding, BindingConfig, BindingId, BindingMode, BindingSystem, BindingTarget, BindingUpdate,
};
pub use error::{
    ActionError, BindingError, DataFlowError, DataSourceError, PipelineError, RepositoryError,
    TransformError,
};
pub use repository::{
    DataPath, DataRepository, DataStore, MemoryStore, PathSegment, RepositoryChange,
};
pub use source::{
    DataSchema, DataSource, DataUpdate, SchemaType, SourceConfig, SourceId, SourceStatus,
    UpdateType,
};
pub use sources::{
    FileFormat, FileSource, FileSourceConfig, HttpSource, HttpSourceConfig, MqttSource,
    MqttSourceConfig, NatsSource, NatsSourceConfig, RedisSource, RedisSourceConfig, TimerSource,
    TimerSourceConfig, WebSocketSource, WebSocketSourceConfig,
};
pub use transform::{
    FilterTransform, MapTransform, Pipeline, SelectTransform, SkipTransform, SortTransform,
    TakeTransform, Transform, TransformContext,
};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// The main data flow engine that orchestrates all data operations.
pub struct DataFlowEngine {
    /// Data repository.
    pub repository: Arc<DataRepository>,
    /// Action system.
    pub action_system: Arc<ActionSystem>,
    /// Binding system.
    pub binding_system: Arc<RwLock<BindingSystem>>,
    /// Active data sources.
    sources: RwLock<HashMap<String, Box<dyn DataSource>>>,
    /// Pipelines by source ID.
    pipelines: RwLock<HashMap<String, Pipeline>>,
}

impl DataFlowEngine {
    /// Creates a new data flow engine.
    pub fn new() -> Self {
        Self {
            repository: Arc::new(DataRepository::new()),
            action_system: Arc::new(ActionSystem::new()),
            binding_system: Arc::new(RwLock::new(BindingSystem::new())),
            sources: RwLock::new(HashMap::new()),
            pipelines: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a data source.
    pub async fn register_source(&self, source: Box<dyn DataSource>) {
        let id = source.id().to_string();
        self.sources.write().await.insert(id, source);
    }

    /// Unregisters a data source.
    pub async fn unregister_source(&self, id: &str) -> Option<Box<dyn DataSource>> {
        self.sources.write().await.remove(id)
    }

    /// Gets a reference to a source by ID.
    pub async fn has_source(&self, id: &str) -> bool {
        self.sources.read().await.contains_key(id)
    }

    /// Lists all source IDs.
    pub async fn source_ids(&self) -> Vec<String> {
        self.sources.read().await.keys().cloned().collect()
    }

    /// Sets a pipeline for a source.
    pub async fn set_pipeline(&self, source_id: &str, pipeline: Pipeline) {
        self.pipelines
            .write()
            .await
            .insert(source_id.to_string(), pipeline);
    }

    /// Starts a source by ID.
    pub async fn start_source(&self, id: &str) -> Result<(), DataSourceError> {
        let mut sources = self.sources.write().await;
        if let Some(source) = sources.get_mut(id) {
            source.start().await
        } else {
            Err(DataSourceError::NotStarted)
        }
    }

    /// Stops a source by ID.
    pub async fn stop_source(&self, id: &str) -> Result<(), DataSourceError> {
        let mut sources = self.sources.write().await;
        if let Some(source) = sources.get_mut(id) {
            source.stop().await
        } else {
            Err(DataSourceError::NotStarted)
        }
    }

    /// Starts all registered sources.
    pub async fn start_all(&self) -> Vec<(String, Result<(), DataSourceError>)> {
        let mut results = Vec::new();
        let mut sources = self.sources.write().await;

        for (id, source) in sources.iter_mut() {
            let result = source.start().await;
            results.push((id.clone(), result));
        }

        results
    }

    /// Stops all registered sources.
    pub async fn stop_all(&self) -> Vec<(String, Result<(), DataSourceError>)> {
        let mut results = Vec::new();
        let mut sources = self.sources.write().await;

        for (id, source) in sources.iter_mut() {
            let result = source.stop().await;
            results.push((id.clone(), result));
        }

        results
    }

    /// Subscribes to updates from a specific source.
    pub async fn subscribe_source(&self, id: &str) -> Option<broadcast::Receiver<DataUpdate>> {
        let sources = self.sources.read().await;
        sources.get(id).map(|source| source.subscribe())
    }

    /// Processes a data update from a source.
    pub async fn process_update(&self, update: DataUpdate) -> Result<(), DataFlowError> {
        let source_id = update.source_id.clone();

        // Apply pipeline if present
        let data = {
            let pipelines = self.pipelines.read().await;
            if let Some(pipeline) = pipelines.get(&source_id) {
                let ctx = TransformContext {
                    source_id: source_id.clone(),
                    timestamp: update.timestamp,
                    variables: HashMap::new(),
                };
                pipeline
                    .execute(update.data, &ctx)
                    .map_err(|e| DataFlowError::Transform {
                        source_id: source_id.clone(),
                        stage: 0,
                        error: TransformError::Expression(e.to_string()),
                    })?
            } else {
                update.data
            }
        };

        // Store in repository
        self.repository
            .update_from_source(&source_id, data)
            .map_err(|e| DataFlowError::Repository {
                path: format!("data.{}", source_id),
                error: e,
            })?;

        Ok(())
    }

    /// Creates a binding and returns its ID.
    pub async fn create_binding(
        &self,
        source: DataPath,
        target: BindingTarget,
        config: BindingConfig,
    ) -> BindingId {
        self.binding_system
            .write()
            .await
            .create_binding(source, target, config)
    }

    /// Removes a binding.
    pub async fn remove_binding(&self, id: BindingId) {
        self.binding_system.write().await.remove_binding(id);
    }
}

impl Default for DataFlowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = DataFlowEngine::new();
        assert!(engine.source_ids().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_source() {
        let engine = DataFlowEngine::new();

        let config = TimerSourceConfig {
            id: "test-timer".into(),
            ..Default::default()
        };
        let source = Box::new(TimerSource::new(config));

        engine.register_source(source).await;
        assert!(engine.has_source("test-timer").await);
    }

    #[tokio::test]
    async fn test_process_update() {
        let engine = DataFlowEngine::new();

        let update = DataUpdate::full("test-source", nemo_config::Value::Integer(42));
        let result = engine.process_update(update).await;

        assert!(result.is_ok());

        let path = DataPath::from_source("test-source");
        let value = engine.repository.get(&path);
        assert_eq!(value, Some(nemo_config::Value::Integer(42)));
    }

    #[tokio::test]
    async fn test_create_binding() {
        let engine = DataFlowEngine::new();

        let source = DataPath::parse("data.test").unwrap();
        let target = BindingTarget::new("component", "value");
        let config = BindingConfig::default();

        let id = engine.create_binding(source, target, config).await;

        let binding_system = engine.binding_system.read().await;
        assert!(binding_system.get_binding(id).is_some());
    }
}
