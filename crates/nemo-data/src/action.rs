//! Action system for data-driven operations.

use crate::error::ActionError;
use crate::repository::{DataPath, RepositoryChange};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Unique identifier for an action.
pub type ActionId = String;

/// Context provided to actions during execution.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// The change that triggered this action (if any).
    pub trigger: Option<RepositoryChange>,
    /// Variables available during execution.
    pub variables: HashMap<String, Value>,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            trigger: None,
            variables: HashMap::new(),
        }
    }
}

/// Trait for executable actions.
#[async_trait]
pub trait Action: Send + Sync {
    /// Executes the action with the given parameters.
    async fn execute(&self, params: Value, context: &ActionContext) -> Result<Value, ActionError>;

    /// Returns the name of this action type.
    fn name(&self) -> &str;
}

/// Direction for threshold triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdDirection {
    /// Trigger when value goes above threshold.
    Above,
    /// Trigger when value goes below threshold.
    Below,
    /// Trigger when value crosses threshold in either direction.
    Cross,
}

/// Condition that triggers an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Trigger when a path changes.
    PathChanged(DataPath),
    /// Trigger when a value crosses a threshold.
    Threshold {
        path: DataPath,
        threshold: Value,
        direction: ThresholdDirection,
    },
    /// Trigger on any update to a path.
    AnyUpdate(DataPath),
}

/// Configuration for an action trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTrigger {
    /// Unique identifier.
    pub id: String,
    /// Condition that activates the trigger.
    pub condition: TriggerCondition,
    /// Action to execute.
    pub action: String,
    /// Parameters to pass to the action.
    pub action_params: Value,
    /// Debounce duration.
    pub debounce: Option<Duration>,
    /// Throttle duration.
    pub throttle: Option<Duration>,
}

/// State for a trigger (for debouncing/throttling).
struct TriggerState {
    last_fired: Option<std::time::Instant>,
    pending: bool,
}

/// The action system manages actions and triggers.
pub struct ActionSystem {
    /// Registered actions.
    actions: RwLock<HashMap<String, Arc<dyn Action>>>,
    /// Configured triggers.
    triggers: RwLock<Vec<ActionTrigger>>,
    /// Trigger states.
    trigger_states: RwLock<HashMap<String, TriggerState>>,
}

impl ActionSystem {
    /// Creates a new action system.
    pub fn new() -> Self {
        Self {
            actions: RwLock::new(HashMap::new()),
            triggers: RwLock::new(Vec::new()),
            trigger_states: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an action.
    pub async fn register_action(&self, name: &str, action: Arc<dyn Action>) {
        let mut actions = self.actions.write().await;
        actions.insert(name.to_string(), action);
    }

    /// Adds a trigger.
    pub async fn add_trigger(&self, trigger: ActionTrigger) {
        let id = trigger.id.clone();
        self.triggers.write().await.push(trigger);
        self.trigger_states.write().await.insert(
            id,
            TriggerState {
                last_fired: None,
                pending: false,
            },
        );
    }

    /// Removes a trigger by ID.
    pub async fn remove_trigger(&self, id: &str) {
        self.triggers.write().await.retain(|t| t.id != id);
        self.trigger_states.write().await.remove(id);
    }

    /// Processes a repository change and fires matching triggers.
    pub async fn on_data_changed(
        &self,
        change: &RepositoryChange,
    ) -> Vec<Result<Value, ActionError>> {
        let mut results = Vec::new();
        let triggers = self.triggers.read().await.clone();

        for trigger in triggers {
            if self.should_fire(&trigger, change).await {
                let context = ActionContext {
                    trigger: Some(change.clone()),
                    variables: HashMap::new(),
                };

                if let Some(action) = self.actions.read().await.get(&trigger.action) {
                    let result = action
                        .execute(trigger.action_params.clone(), &context)
                        .await;
                    results.push(result);

                    // Update trigger state
                    if let Ok(mut states) = self.trigger_states.try_write() {
                        if let Some(state) = states.get_mut(&trigger.id) {
                            state.last_fired = Some(std::time::Instant::now());
                        }
                    }
                } else {
                    results.push(Err(ActionError::NotFound(trigger.action.clone())));
                }
            }
        }

        results
    }

    /// Checks if a trigger should fire based on the change.
    async fn should_fire(&self, trigger: &ActionTrigger, change: &RepositoryChange) -> bool {
        // Check condition
        let matches = match &trigger.condition {
            TriggerCondition::PathChanged(path) => path.matches(&change.path),
            TriggerCondition::AnyUpdate(path) => path.matches(&change.path),
            TriggerCondition::Threshold {
                path,
                threshold,
                direction,
            } => {
                if !path.matches(&change.path) {
                    return false;
                }

                match (&change.old_value, &change.new_value, direction) {
                    (Some(old), Some(new), ThresholdDirection::Above) => {
                        Self::compare_values(old, threshold) != std::cmp::Ordering::Greater
                            && Self::compare_values(new, threshold) == std::cmp::Ordering::Greater
                    }
                    (Some(old), Some(new), ThresholdDirection::Below) => {
                        Self::compare_values(old, threshold) != std::cmp::Ordering::Less
                            && Self::compare_values(new, threshold) == std::cmp::Ordering::Less
                    }
                    (Some(old), Some(new), ThresholdDirection::Cross) => {
                        Self::compare_values(old, threshold) != Self::compare_values(new, threshold)
                    }
                    _ => false,
                }
            }
        };

        if !matches {
            return false;
        }

        // Check throttle/debounce
        if let Ok(states) = self.trigger_states.try_read() {
            if let Some(state) = states.get(&trigger.id) {
                if let Some(last_fired) = state.last_fired {
                    let elapsed = last_fired.elapsed();

                    if let Some(throttle) = trigger.throttle {
                        if elapsed < throttle {
                            return false;
                        }
                    }

                    if let Some(debounce) = trigger.debounce {
                        if elapsed < debounce {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Value::Integer(a), Value::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(a), Value::Integer(b)) => a
                .partial_cmp(&(*b as f64))
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        }
    }

    /// Executes an action directly by name.
    pub async fn execute(
        &self,
        action_name: &str,
        params: Value,
        context: &ActionContext,
    ) -> Result<Value, ActionError> {
        let actions = self.actions.read().await;
        let action = actions
            .get(action_name)
            .ok_or_else(|| ActionError::NotFound(action_name.to_string()))?;
        action.execute(params, context).await
    }

    /// Lists all registered action names.
    pub async fn list_actions(&self) -> Vec<String> {
        self.actions.read().await.keys().cloned().collect()
    }

    /// Lists all trigger IDs.
    pub async fn list_triggers(&self) -> Vec<String> {
        self.triggers
            .read()
            .await
            .iter()
            .map(|t| t.id.clone())
            .collect()
    }
}

impl Default for ActionSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Built-in Actions ----

/// Action that sets data in the repository.
pub struct SetDataAction;

#[async_trait]
impl Action for SetDataAction {
    async fn execute(&self, params: Value, _context: &ActionContext) -> Result<Value, ActionError> {
        // In a real implementation, this would interact with the repository
        // For now, just return the params
        Ok(params)
    }

    fn name(&self) -> &str {
        "set_data"
    }
}

/// Action that logs a message.
pub struct LogAction;

#[async_trait]
impl Action for LogAction {
    async fn execute(&self, params: Value, _context: &ActionContext) -> Result<Value, ActionError> {
        if let Value::Object(obj) = &params {
            if let Some(Value::String(msg)) = obj.get("message") {
                tracing::info!("LogAction: {}", msg);
            }
        }
        Ok(Value::Null)
    }

    fn name(&self) -> &str {
        "log"
    }
}

/// Action that executes a sequence of actions.
pub struct SequenceAction {
    action_system: Arc<ActionSystem>,
}

impl SequenceAction {
    /// Creates a new sequence action.
    pub fn new(action_system: Arc<ActionSystem>) -> Self {
        Self { action_system }
    }
}

#[async_trait]
impl Action for SequenceAction {
    async fn execute(&self, params: Value, context: &ActionContext) -> Result<Value, ActionError> {
        let actions = match params {
            Value::Array(arr) => arr,
            _ => {
                return Err(ActionError::InvalidParams(
                    "Expected array of actions".to_string(),
                ))
            }
        };

        let mut results = Vec::new();

        for action in actions {
            if let Value::Object(obj) = action {
                if let (Some(Value::String(name)), Some(params)) =
                    (obj.get("action"), obj.get("params"))
                {
                    let result = self
                        .action_system
                        .execute(name, params.clone(), context)
                        .await?;
                    results.push(result);
                }
            }
        }

        Ok(Value::Array(results))
    }

    fn name(&self) -> &str {
        "sequence"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_action_system_register() {
        let system = ActionSystem::new();
        system.register_action("log", Arc::new(LogAction)).await;

        let actions = system.list_actions().await;
        assert!(actions.contains(&"log".to_string()));
    }

    #[tokio::test]
    async fn test_log_action() {
        let action = LogAction;
        let mut obj = indexmap::IndexMap::new();
        obj.insert("message".to_string(), Value::String("test".into()));

        let result = action
            .execute(Value::Object(obj), &ActionContext::default())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_path_changed() {
        let system = ActionSystem::new();
        system.register_action("log", Arc::new(LogAction)).await;

        let trigger = ActionTrigger {
            id: "test-trigger".to_string(),
            condition: TriggerCondition::PathChanged(DataPath::parse("data.test").unwrap()),
            action: "log".to_string(),
            action_params: Value::Object(indexmap::IndexMap::new()),
            debounce: None,
            throttle: None,
        };

        system.add_trigger(trigger).await;

        let change = RepositoryChange {
            path: DataPath::parse("data.test").unwrap(),
            old_value: Some(Value::Integer(1)),
            new_value: Some(Value::Integer(2)),
            timestamp: chrono::Utc::now(),
        };

        let results = system.on_data_changed(&change).await;
        assert_eq!(results.len(), 1);
    }
}
