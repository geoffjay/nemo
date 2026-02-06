//! RHAI scripting engine for extension scripts.

use crate::error::ExtensionError;
use nemo_plugin_api::{LogLevel, PluginContext, PluginValue};
use rhai::{Dynamic, Engine, Module, Scope, AST};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the RHAI engine.
#[derive(Debug, Clone)]
pub struct RhaiConfig {
    /// Maximum number of operations before timeout.
    pub max_operations: u64,
    /// Maximum string length.
    pub max_string_size: usize,
    /// Maximum array size.
    pub max_array_size: usize,
    /// Maximum map size.
    pub max_map_size: usize,
    /// Maximum call stack depth.
    pub max_call_stack_depth: usize,
    /// Enabled features.
    pub features: RhaiFeatures,
}

impl Default for RhaiConfig {
    fn default() -> Self {
        Self {
            max_operations: 100_000,
            max_string_size: 64 * 1024,
            max_array_size: 10_000,
            max_map_size: 10_000,
            max_call_stack_depth: 64,
            features: RhaiFeatures::default(),
        }
    }
}

/// Features that can be enabled/disabled in the RHAI engine.
#[derive(Debug, Clone, Default)]
pub struct RhaiFeatures {
    /// Allow file I/O operations.
    pub file_io: bool,
    /// Allow network operations.
    pub network: bool,
    /// Allow system operations.
    pub system: bool,
}

/// A compiled script.
struct CompiledScript {
    /// The compiled AST.
    ast: AST,
    /// Script scope with defined variables.
    scope: Scope<'static>,
}

/// RHAI scripting engine.
pub struct RhaiEngine {
    /// The underlying RHAI engine.
    engine: Engine,
    /// Compiled scripts by ID.
    scripts: HashMap<String, CompiledScript>,
    /// Configuration.
    config: RhaiConfig,
}

impl RhaiEngine {
    /// Creates a new RHAI engine with the given configuration.
    pub fn new(config: RhaiConfig) -> Self {
        let mut engine = Engine::new();

        // Apply limits
        engine.set_max_operations(config.max_operations);
        engine.set_max_string_size(config.max_string_size);
        engine.set_max_array_size(config.max_array_size);
        engine.set_max_map_size(config.max_map_size);
        engine.set_max_call_levels(config.max_call_stack_depth);

        // Register standard functions
        Self::register_standard_functions(&mut engine);

        Self {
            engine,
            scripts: HashMap::new(),
            config,
        }
    }

    /// Registers standard functions available to all scripts.
    fn register_standard_functions(engine: &mut Engine) {
        // Math functions
        engine.register_fn("abs", |x: i64| x.abs());
        engine.register_fn("abs", |x: f64| x.abs());
        engine.register_fn("min", |a: i64, b: i64| a.min(b));
        engine.register_fn("max", |a: i64, b: i64| a.max(b));
        engine.register_fn("min", |a: f64, b: f64| a.min(b));
        engine.register_fn("max", |a: f64, b: f64| a.max(b));
        engine.register_fn("clamp", |x: i64, min: i64, max: i64| x.clamp(min, max));
        engine.register_fn("clamp", |x: f64, min: f64, max: f64| x.clamp(min, max));

        // String functions
        engine.register_fn("trim", |s: &str| s.trim().to_string());
        engine.register_fn("to_upper", |s: &str| s.to_uppercase());
        engine.register_fn("to_lower", |s: &str| s.to_lowercase());
        engine.register_fn("starts_with", |s: &str, prefix: &str| s.starts_with(prefix));
        engine.register_fn("ends_with", |s: &str, suffix: &str| s.ends_with(suffix));
        engine.register_fn("contains", |s: &str, pattern: &str| s.contains(pattern));
        engine.register_fn("replace", |s: &str, from: &str, to: &str| {
            s.replace(from, to)
        });

        // Logging functions (using tracing)
        engine.register_fn("log_debug", |msg: &str| {
            tracing::debug!(target: "rhai_script", "{}", msg);
        });
        engine.register_fn("log_info", |msg: &str| {
            tracing::info!(target: "rhai_script", "{}", msg);
        });
        engine.register_fn("log_warn", |msg: &str| {
            tracing::warn!(target: "rhai_script", "{}", msg);
        });
        engine.register_fn("log_error", |msg: &str| {
            tracing::error!(target: "rhai_script", "{}", msg);
        });

        // Print function for simple output
        engine.register_fn("print", |msg: &str| {
            println!("{}", msg);
        });
    }

    /// Loads and compiles a script.
    pub fn load_script(&mut self, id: &str, source: &str) -> Result<(), ExtensionError> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| ExtensionError::ScriptError {
                script_id: id.to_string(),
                reason: e.to_string(),
            })?;

        let compiled = CompiledScript {
            ast,
            scope: Scope::new(),
        };

        self.scripts.insert(id.to_string(), compiled);
        Ok(())
    }

    /// Reloads a script with new source.
    pub fn reload_script(&mut self, id: &str, source: &str) -> Result<(), ExtensionError> {
        if !self.scripts.contains_key(id) {
            return Err(ExtensionError::NotFound { id: id.to_string() });
        }
        self.load_script(id, source)
    }

    /// Unloads a script.
    pub fn unload_script(&mut self, id: &str) -> Result<(), ExtensionError> {
        self.scripts
            .remove(id)
            .ok_or_else(|| ExtensionError::NotFound { id: id.to_string() })?;
        Ok(())
    }

    /// Calls a function in a script.
    pub fn call<T: Clone + Send + Sync + 'static>(
        &self,
        script_id: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, ExtensionError> {
        let script = self
            .scripts
            .get(script_id)
            .ok_or_else(|| ExtensionError::NotFound {
                id: script_id.to_string(),
            })?;

        self.engine
            .call_fn(&mut script.scope.clone(), &script.ast, function, args)
            .map_err(|e| ExtensionError::ScriptError {
                script_id: script_id.to_string(),
                reason: e.to_string(),
            })
    }

    /// Evaluates an expression.
    pub fn eval<T: Clone + Send + Sync + 'static>(&self, expr: &str) -> Result<T, ExtensionError> {
        self.engine.eval(expr).map_err(|e| e.into())
    }

    /// Evaluates an expression with a scope.
    pub fn eval_with_scope<T: Clone + Send + Sync + 'static>(
        &self,
        scope: &mut Scope,
        expr: &str,
    ) -> Result<T, ExtensionError> {
        self.engine.eval_with_scope(scope, expr).map_err(|e| e.into())
    }

    /// Runs a script and returns the result.
    pub fn run(&self, script_id: &str) -> Result<Dynamic, ExtensionError> {
        let script = self
            .scripts
            .get(script_id)
            .ok_or_else(|| ExtensionError::NotFound {
                id: script_id.to_string(),
            })?;

        self.engine
            .run_ast(&script.ast)
            .map_err(|e| ExtensionError::ScriptError {
                script_id: script_id.to_string(),
                reason: e.to_string(),
            })?;

        Ok(Dynamic::UNIT)
    }

    /// Registers a custom function with no arguments.
    pub fn register_fn_0<R: Clone + Send + Sync + 'static>(
        &mut self,
        name: &str,
        func: impl Fn() -> R + Send + Sync + 'static,
    ) {
        self.engine.register_fn(name, func);
    }

    /// Registers a custom function with one argument.
    pub fn register_fn_1<A: Clone + Send + Sync + 'static, R: Clone + Send + Sync + 'static>(
        &mut self,
        name: &str,
        func: impl Fn(A) -> R + Send + Sync + 'static,
    ) {
        self.engine.register_fn(name, func);
    }

    /// Registers a custom function with two arguments.
    pub fn register_fn_2<
        A: Clone + Send + Sync + 'static,
        B: Clone + Send + Sync + 'static,
        R: Clone + Send + Sync + 'static,
    >(
        &mut self,
        name: &str,
        func: impl Fn(A, B) -> R + Send + Sync + 'static,
    ) {
        self.engine.register_fn(name, func);
    }

    /// Registers a custom module.
    pub fn register_module(&mut self, name: &str, module: Module) {
        self.engine.register_static_module(name, std::rc::Rc::new(module));
    }

    /// Registers the plugin context API.
    pub fn register_context(&mut self, context: Arc<dyn PluginContext>) {
        let ctx = context.clone();
        self.engine
            .register_fn("get_data", move |path: &str| -> Dynamic {
                match ctx.get_data(path) {
                    Some(value) => plugin_value_to_dynamic(value),
                    None => Dynamic::UNIT,
                }
            });

        let ctx = context.clone();
        self.engine
            .register_fn("get_config", move |path: &str| -> Dynamic {
                match ctx.get_config(path) {
                    Some(value) => plugin_value_to_dynamic(value),
                    None => Dynamic::UNIT,
                }
            });

        let ctx = context.clone();
        self.engine.register_fn("log_debug", move |msg: &str| {
            ctx.log(LogLevel::Debug, msg);
        });

        let ctx = context.clone();
        self.engine.register_fn("log_info", move |msg: &str| {
            ctx.log(LogLevel::Info, msg);
        });

        let ctx = context.clone();
        self.engine.register_fn("log_warn", move |msg: &str| {
            ctx.log(LogLevel::Warn, msg);
        });

        let ctx = context;
        self.engine.register_fn("log_error", move |msg: &str| {
            ctx.log(LogLevel::Error, msg);
        });
    }

    /// Lists all loaded script IDs.
    pub fn list_scripts(&self) -> Vec<String> {
        self.scripts.keys().cloned().collect()
    }

    /// Returns the configuration.
    pub fn config(&self) -> &RhaiConfig {
        &self.config
    }
}

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new(RhaiConfig::default())
    }
}

/// Converts a PluginValue to a RHAI Dynamic.
fn plugin_value_to_dynamic(value: PluginValue) -> Dynamic {
    match value {
        PluginValue::Null => Dynamic::UNIT,
        PluginValue::Bool(b) => Dynamic::from(b),
        PluginValue::Integer(i) => Dynamic::from(i),
        PluginValue::Float(f) => Dynamic::from(f),
        PluginValue::String(s) => Dynamic::from(s),
        PluginValue::Array(arr) => {
            let vec: Vec<Dynamic> = arr.into_iter().map(plugin_value_to_dynamic).collect();
            Dynamic::from(vec)
        }
        PluginValue::Object(obj) => {
            let map: rhai::Map = obj
                .into_iter()
                .map(|(k, v)| (k.into(), plugin_value_to_dynamic(v)))
                .collect();
            Dynamic::from(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        assert!(engine.scripts.is_empty());
    }

    #[test]
    fn test_eval_expression() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result: i64 = engine.eval("40 + 2").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_load_and_call() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());

        let script = r#"
            fn add(a, b) {
                a + b
            }
        "#;

        engine.load_script("test", script).unwrap();
        let result: i64 = engine.call("test", "add", (10_i64, 5_i64)).unwrap();
        assert_eq!(result, 15);
    }

    #[test]
    fn test_custom_functions() {
        let engine = RhaiEngine::new(RhaiConfig::default());

        let result: i64 = engine.eval("abs(-42)").unwrap();
        assert_eq!(result, 42);

        let result: String = engine.eval("to_upper(\"hello\")").unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_script_not_found() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result: Result<i64, _> = engine.call("nonexistent", "func", ());
        assert!(matches!(result, Err(ExtensionError::NotFound { .. })));
    }
}
