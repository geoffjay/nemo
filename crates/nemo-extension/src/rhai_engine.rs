//! RHAI scripting engine for extension scripts.

use crate::error::ExtensionError;
use nemo_plugin_api::{LogLevel, PluginContext, PluginValue};
use rhai::packages::Package;
use rhai::{Dynamic, Engine, Module, Scope, AST};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle as TokioHandle;

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
    /// Allow file I/O operations (rhai-fs).
    pub file_io: bool,
    /// Allow network operations.
    pub network: bool,
    /// Allow system operations (rhai-env, rhai-process).
    pub system: bool,
    /// Allow scientific computing functions (rhai-sci).
    pub science: bool,
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

        // Register standard functions (math, string, conversion, logging),
        // the rhai-chrono package (pure — no host I/O), JSON helpers, and —
        // only when `features.file_io` is enabled — the rhai-fs package.
        Self::register_standard_functions(&mut engine, &config);

        Self {
            engine,
            scripts: HashMap::new(),
            config,
        }
    }

    /// Registers standard functions available to all scripts.
    fn register_standard_functions(engine: &mut Engine, config: &RhaiConfig) {
        // rhai-chrono: pure date/time arithmetic, no host I/O. Always registered.
        rhai_chrono::ChronoPackage::new().register_into_engine(engine);
        // Math functions
        engine.register_fn("abs", |x: i64| x.abs());
        engine.register_fn("abs", |x: f64| x.abs());
        engine.register_fn("min", |a: i64, b: i64| a.min(b));
        engine.register_fn("max", |a: i64, b: i64| a.max(b));
        engine.register_fn("min", |a: f64, b: f64| a.min(b));
        engine.register_fn("max", |a: f64, b: f64| a.max(b));
        engine.register_fn("clamp", |x: i64, min: i64, max: i64| x.clamp(min, max));
        engine.register_fn("clamp", |x: f64, min: f64, max: f64| x.clamp(min, max));
        engine.register_fn("floor", |x: f64| x.floor());
        engine.register_fn("ceil", |x: f64| x.ceil());
        engine.register_fn("round", |x: f64| x.round());
        engine.register_fn("sqrt", |x: f64| x.sqrt());
        engine.register_fn("pow", |x: f64, y: f64| x.powf(y));

        // Type conversion functions
        engine.register_fn("parse_float", |s: &str| -> f64 {
            s.parse::<f64>().unwrap_or(0.0)
        });
        engine.register_fn("parse_int", |s: &str| -> i64 {
            s.parse::<i64>().unwrap_or(0)
        });
        engine.register_fn("to_string", |x: i64| x.to_string());
        engine.register_fn("to_string", |x: f64| {
            // Format nicely: remove trailing zeros for whole numbers
            if x == x.floor() && x.abs() < 1e15 {
                format!("{}", x as i64)
            } else {
                format!("{}", x)
            }
        });
        engine.register_fn("to_int", |x: f64| x as i64);
        engine.register_fn("to_float", |x: i64| x as f64);

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

        // JSON helpers. Rhai has no built-in JSON, so expose two functions
        // backed by serde_json (already a workspace dep). `json_parse` returns
        // a Dynamic (map/array/scalar); `json_stringify` serializes a Dynamic.
        engine.register_fn("json_parse", |s: &str| -> Dynamic {
            match serde_json::from_str::<serde_json::Value>(s) {
                Ok(value) => json_to_dynamic(value),
                Err(_) => Dynamic::UNIT,
            }
        });
        engine.register_fn("json_stringify", |d: Dynamic| -> String {
            let json = dynamic_to_json(&d);
            serde_json::to_string(&json).unwrap_or_default()
        });

        // rhai-fs: filesystem read/write. Only register when the app opts in
        // via `RhaiFeatures.file_io` — this preserves the default sandbox
        // (no host I/O) and lets scripts touch the disk only when the app
        // explicitly enables it.
        if config.features.file_io {
            rhai_fs::FilesystemPackage::new().register_into_engine(engine);
        }

        // rhai-env: environment variable access. Gated by `system` and the
        // `pkg-env` cargo feature. Grants scripts read/write access to
        // process environment variables — opt-in.
        #[cfg(feature = "pkg-env")]
        if config.features.system {
            rhai_env::EnvironmentPackage::new().register_into_engine(engine);
        }
        // If the app asks for `system` but this binary was built without the
        // `pkg-env` cargo feature, `env()`/`envs()` won't exist. Warn loudly
        // rather than let scripts fail later with a cryptic "Function not
        // found: env".
        #[cfg(not(feature = "pkg-env"))]
        if config.features.system {
            tracing::warn!(
                "Script feature 'system' is enabled but nemo-extension was built \
                 without the 'pkg-env' cargo feature — env()/envs()/set_env() are \
                 unavailable. Rebuild with `--features nemo/pkg-env` (or \
                 `nemo-extension/pkg-env`)."
            );
        }

        // rhai-sci: scientific computing (mean, std, linspace, matrix ops,
        // etc.). Gated by `science` and the `pkg-sci` cargo feature. Pure
        // computation, but heavy dependency tree, so behind a cargo feature.
        #[cfg(feature = "pkg-sci")]
        if config.features.science {
            rhai_sci::SciPackage::new().register_into_engine(engine);
        }
        #[cfg(not(feature = "pkg-sci"))]
        if config.features.science {
            tracing::warn!(
                "Script feature 'science' is enabled but nemo-extension was built \
                 without the 'pkg-sci' cargo feature — mean()/std()/median()/… are \
                 unavailable. Rebuild with `--features nemo/pkg-sci` (or \
                 `nemo-extension/pkg-sci`)."
            );
        }

        // rhai-process: subprocess execution. Gated by `system` and the
        // `pkg-process` cargo feature. The most dangerous package — spawns
        // external processes. Strictly opt-in.
        #[cfg(feature = "pkg-process")]
        if config.features.system {
            rhai_process::ProcessPackage::new(rhai_process::Config::default())
                .register_into_engine(engine);
        }
        #[cfg(not(feature = "pkg-process"))]
        if config.features.system {
            tracing::warn!(
                "Script feature 'system' is enabled but nemo-extension was built \
                 without the 'pkg-process' cargo feature — cmd(...) subprocess \
                 execution is unavailable. Rebuild with `--features nemo/pkg-process` \
                 (or `nemo-extension/pkg-process`)."
            );
        }
    }

    /// Loads and compiles a script.
    ///
    /// After compilation the script's top-level statements (`let`/`const`
    /// declarations, imports, etc.) are run once against a fresh scope to
    /// seed the script's persistent state. That scope is reused on every
    /// [`call`](Self::call), so module-level variables persist across
    /// handler invocations (e.g. the `tasks_loaded` flag in the task-list
    /// example). Without this step, `call_fn` gets an empty scope and any
    /// reference to a top-level variable fails with "Variable not found".
    pub fn load_script(&mut self, id: &str, source: &str) -> Result<(), ExtensionError> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| ExtensionError::ScriptError {
                script_id: id.to_string(),
                reason: e.to_string(),
            })?;

        let mut scope = Scope::new();
        self.engine
            .run_ast_with_scope(&mut scope, &ast)
            .map_err(|e| ExtensionError::ScriptError {
                script_id: id.to_string(),
                reason: e.to_string(),
            })?;

        let compiled = CompiledScript { ast, scope };

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
    ///
    /// Uses the script's persistent scope (seeded at load time and updated
    /// on every call), so module-level variables survive across invocations.
    pub fn call<T: Clone + Send + Sync + 'static>(
        &mut self,
        script_id: &str,
        function: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T, ExtensionError> {
        let script = self
            .scripts
            .get_mut(script_id)
            .ok_or_else(|| ExtensionError::NotFound {
                id: script_id.to_string(),
            })?;

        self.engine
            .call_fn(&mut script.scope, &script.ast, function, args)
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
        self.engine
            .eval_with_scope(scope, expr)
            .map_err(|e| e.into())
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
        self.engine
            .register_static_module(name, std::rc::Rc::new(module));
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
            .register_fn("set_data", move |path: &str, value: Dynamic| {
                let plugin_value = dynamic_to_plugin_value(value);
                if let Err(e) = ctx.set_data(path, plugin_value) {
                    tracing::warn!("Failed to set data: {}", e);
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

        let ctx = context.clone();
        self.engine.register_fn("log_error", move |msg: &str| {
            ctx.log(LogLevel::Error, msg);
        });

        // Component property functions
        let ctx = context.clone();
        self.engine.register_fn(
            "get_component_property",
            move |component_id: &str, property: &str| -> Dynamic {
                match ctx.get_component_property(component_id, property) {
                    Some(value) => plugin_value_to_dynamic(value),
                    None => Dynamic::UNIT,
                }
            },
        );

        let ctx = context.clone();
        self.engine.register_fn(
            "set_component_property",
            move |component_id: &str, property: &str, value: Dynamic| {
                let plugin_value = dynamic_to_plugin_value(value);
                if let Err(e) = ctx.set_component_property(component_id, property, plugin_value) {
                    tracing::warn!("Failed to set component property: {}", e);
                }
            },
        );

        // Convenience wrappers for common properties
        let ctx = context.clone();
        self.engine
            .register_fn("get_component_label", move |component_id: &str| -> String {
                ctx.get_component_property(component_id, "label")
                    .map(plugin_value_to_string)
                    .unwrap_or_default()
            });

        let ctx = context.clone();
        self.engine
            .register_fn("get_component_text", move |component_id: &str| -> String {
                ctx.get_component_property(component_id, "text")
                    .map(plugin_value_to_string)
                    .unwrap_or_default()
            });

        let ctx = context.clone();
        self.engine.register_fn(
            "set_component_text",
            move |component_id: &str, text: &str| {
                if let Err(e) = ctx.set_component_property(
                    component_id,
                    "text",
                    PluginValue::String(text.to_string()),
                ) {
                    tracing::warn!("Failed to set component text: {}", e);
                }
            },
        );

        let ctx = context.clone();
        self.engine.register_fn(
            "set_component_label",
            move |component_id: &str, label: &str| {
                if let Err(e) = ctx.set_component_property(
                    component_id,
                    "label",
                    PluginValue::String(label.to_string()),
                ) {
                    tracing::warn!("Failed to set component label: {}", e);
                }
            },
        );

        // Router navigation. `navigate(path)` targets the primary router;
        // `navigate(router, path)` targets an explicit one. `back`/`forward`
        // move through history. All are deferred (applied off the extension
        // lock), so calling them from inside a handler is safe.
        let ctx = context.clone();
        self.engine.register_fn("navigate", move |path: &str| {
            if let Err(e) = ctx.navigate(None, path) {
                tracing::warn!("navigate failed: {}", e);
            }
        });

        let ctx = context.clone();
        self.engine
            .register_fn("navigate", move |router: &str, path: &str| {
                if let Err(e) = ctx.navigate(Some(router), path) {
                    tracing::warn!("navigate failed: {}", e);
                }
            });

        let ctx = context.clone();
        self.engine.register_fn("back", move || {
            if let Err(e) = ctx.back(None) {
                tracing::warn!("back failed: {}", e);
            }
        });

        let ctx = context.clone();
        self.engine.register_fn("back", move |router: &str| {
            if let Err(e) = ctx.back(Some(router)) {
                tracing::warn!("back failed: {}", e);
            }
        });

        let ctx = context.clone();
        self.engine.register_fn("forward", move || {
            if let Err(e) = ctx.forward(None) {
                tracing::warn!("forward failed: {}", e);
            }
        });

        let ctx = context;
        self.engine.register_fn("forward", move |router: &str| {
            if let Err(e) = ctx.forward(Some(router)) {
                tracing::warn!("forward failed: {}", e);
            }
        });
    }

    /// Registers HTTP request functions (`http_get`, `http_post`, `http_put`,
    /// `http_delete`) that allow RHAI scripts to make synchronous HTTP calls.
    ///
    /// These functions block the calling thread while the request executes on
    /// the provided tokio runtime. They are intended for use from UI event
    /// handlers (e.g., `on-click`) which run on the main/GPUI thread, outside
    /// of any async context.
    ///
    /// # Functions registered
    ///
    /// - `http_get(url: &str) -> Dynamic` — GET request, returns parsed JSON or string
    /// - `http_get(url: &str, headers: Map) -> Dynamic` — GET with request headers
    /// - `http_post(url: &str, body: &str) -> Dynamic` — POST with JSON body
    /// - `http_post(url: &str, body: &str, headers: Map) -> Dynamic` — POST with headers
    /// - `http_put(url: &str, body: &str) -> Dynamic` — PUT with JSON body
    /// - `http_put(url: &str, body: &str, headers: Map) -> Dynamic` — PUT with headers
    /// - `http_delete(url: &str) -> Dynamic` — DELETE request
    /// - `http_delete(url: &str, headers: Map) -> Dynamic` — DELETE with headers
    ///
    /// The `headers` map lets scripts send arbitrary request headers, e.g.
    /// `http_get(url, #{ "Authorization": "Bearer " + token })`. A caller-supplied
    /// `Content-Type` overrides the JSON default applied to request bodies.
    ///
    /// All functions return a map with `{status, body, ok}` on success, or
    /// a map with `{error}` on failure.
    pub fn register_http_functions(&mut self, handle: TokioHandle) {
        let client = Arc::new(reqwest::Client::new());

        // http_get(url) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine
            .register_fn("http_get", move |url: &str| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::GET, url, None, None)
            });

        // http_get(url, headers) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine.register_fn(
            "http_get",
            move |url: &str, headers: rhai::Map| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::GET, url, None, Some(headers))
            },
        );

        // http_post(url, body) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine
            .register_fn("http_post", move |url: &str, body: &str| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::POST, url, Some(body), None)
            });

        // http_post(url, body, headers) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine.register_fn(
            "http_post",
            move |url: &str, body: &str, headers: rhai::Map| -> Dynamic {
                execute_http_request(
                    &h,
                    &c,
                    reqwest::Method::POST,
                    url,
                    Some(body),
                    Some(headers),
                )
            },
        );

        // http_put(url, body) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine
            .register_fn("http_put", move |url: &str, body: &str| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::PUT, url, Some(body), None)
            });

        // http_put(url, body, headers) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine.register_fn(
            "http_put",
            move |url: &str, body: &str, headers: rhai::Map| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::PUT, url, Some(body), Some(headers))
            },
        );

        // http_delete(url) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine
            .register_fn("http_delete", move |url: &str| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::DELETE, url, None, None)
            });

        // http_delete(url, headers) -> Dynamic
        let h = handle.clone();
        let c = client.clone();
        self.engine.register_fn(
            "http_delete",
            move |url: &str, headers: rhai::Map| -> Dynamic {
                execute_http_request(&h, &c, reqwest::Method::DELETE, url, None, Some(headers))
            },
        );
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

/// Converts a PluginValue to a String, handling all variant types.
fn plugin_value_to_string(value: PluginValue) -> String {
    match value {
        PluginValue::String(s) => s,
        PluginValue::Integer(i) => i.to_string(),
        PluginValue::Float(f) => f.to_string(),
        PluginValue::Bool(b) => b.to_string(),
        PluginValue::Null => String::new(),
        PluginValue::Array(_) | PluginValue::Object(_) => format!("{:?}", value),
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

/// Converts a RHAI Dynamic to a PluginValue.
fn dynamic_to_plugin_value(value: Dynamic) -> PluginValue {
    if value.is_unit() {
        PluginValue::Null
    } else if value.is_bool() {
        PluginValue::Bool(value.as_bool().unwrap_or(false))
    } else if value.is_int() {
        PluginValue::Integer(value.as_int().unwrap_or(0))
    } else if value.is_float() {
        PluginValue::Float(value.as_float().unwrap_or(0.0))
    } else if value.is_string() {
        PluginValue::String(value.into_string().unwrap_or_default())
    } else if value.is_array() {
        let arr: Vec<Dynamic> = value.into_array().unwrap_or_default();
        PluginValue::Array(arr.into_iter().map(dynamic_to_plugin_value).collect())
    } else if value.is_map() {
        let map: rhai::Map = value.cast();
        let obj: indexmap::IndexMap<String, PluginValue> = map
            .into_iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_plugin_value(v)))
            .collect();
        PluginValue::Object(obj)
    } else {
        // Try to convert to string as fallback
        PluginValue::String(value.to_string())
    }
}

/// Execute an HTTP request synchronously by blocking on the tokio runtime.
///
/// Returns a RHAI map with `{status, body, ok}` on success or `{error}` on failure.
/// If the response body is valid JSON, `body` is the parsed structure; otherwise
/// it is the raw response string.
/// Execute an HTTP request synchronously by blocking on the tokio runtime.
///
/// Returns a RHAI map with `{status, body, ok}` on success or `{error}` on failure.
/// If the response body is valid JSON, `body` is the parsed structure; otherwise
/// it is the raw response string.
fn execute_http_request(
    handle: &TokioHandle,
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    body: Option<&str>,
    headers: Option<rhai::Map>,
) -> Dynamic {
    let url_string = url.to_string();
    let body = body.map(|s| s.to_string());
    let client = client.clone();
    let method_clone = method.clone();

    // Flatten the header map to owned string pairs before crossing the async
    // boundary (rhai::Map/Dynamic are not `Send`). Non-string values are
    // stringified so callers can pass e.g. numeric header values.
    let header_pairs: Vec<(String, String)> = headers
        .map(|map| {
            map.into_iter()
                .map(|(k, v)| {
                    let value = if v.is_string() {
                        v.into_string().unwrap_or_default()
                    } else {
                        v.to_string()
                    };
                    (k.to_string(), value)
                })
                .collect()
        })
        .unwrap_or_default();

    let has_content_type = header_pairs
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));

    let result = handle.block_on(async move {
        let mut builder = client.request(method_clone, &url_string);
        if let Some(b) = body {
            // Default the body content type to JSON unless the caller set it.
            if !has_content_type {
                builder = builder.header("Content-Type", "application/json");
            }
            builder = builder.body(b);
        }
        for (key, value) in header_pairs {
            builder = builder.header(key, value);
        }
        builder.send().await
    });

    match result {
        Ok(response) => {
            let status = response.status().as_u16() as i64;
            let ok = response.status().is_success();

            let response_body = handle.block_on(async { response.text().await });
            match response_body {
                Ok(text) => {
                    let body_dynamic = match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json) => json_to_dynamic(json),
                        Err(_) => Dynamic::from(text),
                    };

                    let mut map = rhai::Map::new();
                    map.insert("status".into(), Dynamic::from(status));
                    map.insert("body".into(), body_dynamic);
                    map.insert("ok".into(), Dynamic::from(ok));
                    Dynamic::from(map)
                }
                Err(e) => {
                    tracing::warn!("HTTP {} {} - failed to read body: {}", method, url, e);
                    let mut map = rhai::Map::new();
                    map.insert("error".into(), Dynamic::from(e.to_string()));
                    Dynamic::from(map)
                }
            }
        }
        Err(e) => {
            tracing::warn!("HTTP {} {} failed: {}", method, url, e);
            let mut map = rhai::Map::new();
            map.insert("error".into(), Dynamic::from(e.to_string()));
            Dynamic::from(map)
        }
    }
}

/// Convert a `serde_json::Value` to a RHAI `Dynamic`.
fn json_to_dynamic(value: serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s),
        serde_json::Value::Array(arr) => {
            let vec: Vec<Dynamic> = arr.into_iter().map(json_to_dynamic).collect();
            Dynamic::from(vec)
        }
        serde_json::Value::Object(obj) => {
            let map: rhai::Map = obj
                .into_iter()
                .map(|(k, v)| (k.into(), json_to_dynamic(v)))
                .collect();
            Dynamic::from(map)
        }
    }
}

/// Convert a RHAI `Dynamic` to a `serde_json::Value` for serialization.
fn dynamic_to_json(value: &Dynamic) -> serde_json::Value {
    if value.is_unit() {
        serde_json::Value::Null
    } else if value.is_bool() {
        serde_json::Value::Bool(value.as_bool().unwrap_or(false))
    } else if value.is_int() {
        serde_json::Value::Number(value.as_int().unwrap_or(0).into())
    } else if let Ok(f) = value.as_float() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if value.is_string() {
        serde_json::Value::String(value.clone().into_string().unwrap_or_default())
    } else if value.is_array() {
        let arr: Vec<serde_json::Value> = value
            .clone()
            .into_array()
            .unwrap_or_default()
            .iter()
            .map(dynamic_to_json)
            .collect();
        serde_json::Value::Array(arr)
    } else if value.is_map() {
        let map: rhai::Map = value.clone().cast();
        let obj: serde_json::Map<String, serde_json::Value> = map
            .into_iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_json(&v)))
            .collect();
        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::String(value.to_string())
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
    fn test_module_state_persists_across_calls() {
        // Regression: scripts with top-level `let` declarations (like the
        // task-list example's `tasks_loaded` flag) must be callable. The
        // scope is seeded at load time and persists across `call`s.
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        let script = r#"
            let counter = 0;
            let initialized = false;

            fn bump() {
                if !initialized {
                    counter = 100;
                    initialized = true;
                }
                counter += 1;
                counter
            }
        "#;
        engine.load_script("s", script).unwrap();

        assert_eq!(engine.call::<i64>("s", "bump", ()).unwrap(), 101);
        assert_eq!(engine.call::<i64>("s", "bump", ()).unwrap(), 102);
        assert_eq!(engine.call::<i64>("s", "bump", ()).unwrap(), 103);
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
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        let result: Result<i64, _> = engine.call("nonexistent", "func", ());
        assert!(matches!(result, Err(ExtensionError::NotFound { .. })));
    }

    // ── Script lifecycle ──────────────────────────────────────────────

    #[test]
    fn test_reload_script() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.load_script("s", "fn val() { 1 }").unwrap();
        assert_eq!(engine.call::<i64>("s", "val", ()).unwrap(), 1);

        engine.reload_script("s", "fn val() { 2 }").unwrap();
        assert_eq!(engine.call::<i64>("s", "val", ()).unwrap(), 2);
    }

    #[test]
    fn test_reload_nonexistent_errors() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        assert!(engine.reload_script("missing", "fn x() {}").is_err());
    }

    #[test]
    fn test_unload_script() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.load_script("s", "fn x() { 1 }").unwrap();
        assert_eq!(engine.list_scripts().len(), 1);

        engine.unload_script("s").unwrap();
        assert!(engine.list_scripts().is_empty());
    }

    #[test]
    fn test_unload_nonexistent_errors() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        assert!(engine.unload_script("missing").is_err());
    }

    #[test]
    fn test_list_scripts() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.load_script("a", "fn x() {}").unwrap();
        engine.load_script("b", "fn y() {}").unwrap();

        let mut ids = engine.list_scripts();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn test_compile_error() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        let result = engine.load_script("bad", "fn broken( {");
        assert!(result.is_err());
    }

    #[test]
    fn test_run_script() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.load_script("s", "let x = 42;").unwrap();
        let result = engine.run("s");
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_nonexistent_errors() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        assert!(engine.run("missing").is_err());
    }

    // ── Built-in functions ────────────────────────────────────────────

    #[test]
    fn test_math_functions() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        assert_eq!(engine.eval::<i64>("min(3, 7)").unwrap(), 3);
        assert_eq!(engine.eval::<i64>("max(3, 7)").unwrap(), 7);
        assert_eq!(engine.eval::<i64>("clamp(10, 0, 5)").unwrap(), 5);
        assert_eq!(engine.eval::<i64>("abs(-5)").unwrap(), 5);
    }

    #[test]
    fn test_string_functions() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        assert_eq!(engine.eval::<String>(r#"trim("  hi  ")"#).unwrap(), "hi");
        assert_eq!(engine.eval::<String>(r#"to_upper("abc")"#).unwrap(), "ABC");
        assert_eq!(engine.eval::<String>(r#"to_lower("ABC")"#).unwrap(), "abc");
        assert!(engine
            .eval::<bool>(r#"starts_with("hello", "he")"#)
            .unwrap());
        assert!(engine.eval::<bool>(r#"ends_with("hello", "lo")"#).unwrap());
    }

    #[test]
    fn test_type_conversions() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        assert_eq!(engine.eval::<i64>(r#"parse_int("42")"#).unwrap(), 42);
        assert!((engine.eval::<f64>(r#"parse_float("1.5")"#).unwrap() - 1.5).abs() < f64::EPSILON);
        assert_eq!(engine.eval::<i64>("to_int(3.7)").unwrap(), 3);
    }

    // ── Eval with scope ───────────────────────────────────────────────

    #[test]
    fn test_eval_with_scope() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let mut scope = rhai::Scope::new();
        scope.push("x", 10_i64);
        let result: i64 = engine.eval_with_scope(&mut scope, "x + 5").unwrap();
        assert_eq!(result, 15);
    }

    // ── Config access ─────────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let config = RhaiConfig::default();
        assert_eq!(config.max_operations, 100_000);
        assert_eq!(config.max_call_stack_depth, 64);
        assert!(!config.features.file_io);
        assert!(!config.features.network);
    }

    // ── Custom function registration ──────────────────────────────────

    #[test]
    fn test_register_custom_fn() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.register_fn_0("get_magic", || 42_i64);
        assert_eq!(engine.eval::<i64>("get_magic()").unwrap(), 42);
    }

    #[test]
    fn test_register_custom_fn_with_args() {
        let mut engine = RhaiEngine::new(RhaiConfig::default());
        engine.register_fn_2("add_custom", |a: i64, b: i64| a + b);
        assert_eq!(engine.eval::<i64>("add_custom(3, 4)").unwrap(), 7);
    }

    // ── JSON helpers ─────────────────────────────────────────────────

    #[test]
    fn test_json_parse_object() {
        // rhai::Map is a BTreeMap, so key order is lexicographic, not
        // insertion order. Round-trip through json_stringify and check
        // both keys rather than the exact string. Use a backticked string
        // to avoid rhai's parser choking on inner double-quotes.
        let engine = RhaiEngine::new(RhaiConfig::default());
        let script = r#"
            let s = `{"name":"Alice","age":42}`;
            json_stringify(json_parse(s))
        "#;
        let result: String = engine.eval(script).unwrap();
        assert!(
            result.contains(r#""name":"Alice""#),
            "expected name field, got: {result}"
        );
        assert!(
            result.contains(r#""age":42"#),
            "expected age field, got: {result}"
        );
    }

    #[test]
    fn test_json_parse_array() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result: String = engine
            .eval(r#"json_stringify(json_parse(`[1, 2, 3]`))"#)
            .unwrap();
        assert_eq!(result, r#"[1,2,3]"#);
    }

    #[test]
    fn test_json_parse_invalid_returns_unit() {
        // Invalid JSON yields () which json_stringify renders as "null".
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result: String = engine
            .eval(r#"json_stringify(json_parse("not json"))"#)
            .unwrap();
        assert_eq!(result, "null");
    }

    #[test]
    fn test_json_stringify_roundtrip() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let script = r#"
            let s = `{"x":1,"y":"two"}`;
            json_stringify(json_parse(s))
        "#;
        let result: String = engine.eval(script).unwrap();
        assert_eq!(result, r#"{"x":1,"y":"two"}"#);
    }

    // ── rhai-chrono package ───────────────────────────────────────────

    #[test]
    fn test_chrono_package_available() {
        // rhai-chrono is registered unconditionally; verify a basic
        // date-time operation succeeds. Functions are registered globally
        // (no module prefix), e.g. `datetime_utc()`, `datetime_now()`.
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result: String = engine
            .eval(r#"let dt = datetime_utc(); dt.to_string()"#)
            .unwrap();
        // RFC3339 string, e.g. "2026-07-14T12:34:56.789+00:00"
        assert!(
            result.contains("T") && result.contains("+00:00"),
            "expected RFC3339 UTC, got: {result}"
        );
    }

    // ── rhai-fs package (gated by features.file_io) ────────────────────

    #[test]
    fn test_fs_not_available_by_default() {
        // Without file_io, rhai-fs functions must not be registered.
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result = engine.eval::<String>(r#"open_file("Cargo.toml").read_string()"#);
        assert!(result.is_err(), "rhai-fs should be disabled by default");
    }

    #[test]
    fn test_fs_available_when_file_io_enabled() {
        let config = RhaiConfig {
            features: RhaiFeatures {
                file_io: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = RhaiEngine::new(config);
        let result: String = engine
            .eval(r#"open_file("Cargo.toml", "r").read_string()"#)
            .unwrap();
        assert!(result.contains("[package]"), "should read Cargo.toml");
    }

    // ── rhai-env package (gated by features.system + pkg-env cargo feature) ──

    #[cfg(feature = "pkg-env")]
    #[test]
    fn test_env_not_available_without_system_feature() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result = engine.eval::<String>(r#"env("HOME")"#);
        assert!(
            result.is_err(),
            "rhai-env should be disabled without system feature"
        );
    }

    #[cfg(feature = "pkg-env")]
    #[test]
    fn test_env_available_when_system_enabled() {
        let config = RhaiConfig {
            features: RhaiFeatures {
                system: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = RhaiEngine::new(config);
        // `env("PATH")` should return the PATH env var (always set in tests).
        let result: String = engine.eval(r#"env("PATH")"#).unwrap();
        assert!(!result.is_empty(), "should read PATH env var");
    }

    // ── rhai-sci package (gated by features.science + pkg-sci cargo feature) ─

    #[cfg(feature = "pkg-sci")]
    #[test]
    fn test_sci_not_available_without_science_feature() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result = engine.eval::<f64>(r#"mean([1.0, 2.0, 3.0])"#);
        assert!(
            result.is_err(),
            "rhai-sci should be disabled without science feature"
        );
    }

    #[cfg(feature = "pkg-sci")]
    #[test]
    fn test_sci_available_when_science_enabled() {
        let config = RhaiConfig {
            features: RhaiFeatures {
                science: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = RhaiEngine::new(config);
        let result: f64 = engine.eval(r#"mean([1.0, 2.0, 3.0])"#).unwrap();
        assert!(
            (result - 2.0).abs() < 1e-10,
            "mean should be 2.0, got {result}"
        );
    }

    // ── rhai-process package (gated by features.system + pkg-process feature) ─

    #[cfg(feature = "pkg-process")]
    #[test]
    fn test_process_not_available_without_system_feature() {
        let engine = RhaiEngine::new(RhaiConfig::default());
        let result = engine.eval::<String>(r#"cmd(["echo", "hello"]).build().run().stdout"#);
        assert!(
            result.is_err(),
            "rhai-process should be disabled without system feature"
        );
    }

    #[cfg(feature = "pkg-process")]
    #[test]
    fn test_process_available_when_system_enabled() {
        let config = RhaiConfig {
            features: RhaiFeatures {
                system: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = RhaiEngine::new(config);
        let result: String = engine
            .eval(r#"cmd(["echo", "hello"]).build().run().stdout"#)
            .unwrap();
        assert!(result.contains("hello"), "echo should output hello");
    }

    // ── Example scripts (end-to-end) ──────────────────────────────────

    /// Minimal in-memory `PluginContext` for driving example handlers:
    /// a key/value data store (get_data/set_data) and a component-property
    /// store (get/set_component_property). Enough to exercise the Rhai host
    /// API without a running app.
    #[derive(Default)]
    struct MockContext {
        data: std::sync::Mutex<HashMap<String, PluginValue>>,
        props: std::sync::Mutex<HashMap<(String, String), PluginValue>>,
    }

    impl nemo_plugin_api::PluginContext for MockContext {
        fn get_data(&self, path: &str) -> Option<PluginValue> {
            self.data.lock().unwrap().get(path).cloned()
        }
        fn set_data(
            &self,
            path: &str,
            value: PluginValue,
        ) -> Result<(), nemo_plugin_api::PluginError> {
            self.data.lock().unwrap().insert(path.to_string(), value);
            Ok(())
        }
        fn emit_event(&self, _event_type: &str, _payload: PluginValue) {}
        fn get_config(&self, _path: &str) -> Option<PluginValue> {
            None
        }
        fn log(&self, _level: LogLevel, _message: &str) {}
        fn get_component_property(
            &self,
            component_id: &str,
            property: &str,
        ) -> Option<PluginValue> {
            self.props
                .lock()
                .unwrap()
                .get(&(component_id.to_string(), property.to_string()))
                .cloned()
        }
        fn set_component_property(
            &self,
            component_id: &str,
            property: &str,
            value: PluginValue,
        ) -> Result<(), nemo_plugin_api::PluginError> {
            self.props
                .lock()
                .unwrap()
                .insert((component_id.to_string(), property.to_string()), value);
            Ok(())
        }
    }

    #[test]
    fn test_task_list_handlers_end_to_end() {
        // Drive the real task-list handlers against a mock context. This is the
        // regression guard for the example's whole lifecycle: on_load creates an
        // empty file and starts empty (no baked-in defaults), submit_add reads
        // the inputs' live `value` and persists, toggle/delete key off the typed
        // row number, and the table `data` array + header count stay in sync
        // with disk. Persistence targets a temp file so the repo is untouched.
        let raw = include_str!("../../../examples/task-list/scripts/handlers.rhai");
        let tmp = tempfile::tempdir().unwrap();
        let data_file = tmp.path().join("tasks.json");
        let script = raw.replace(
            "\"examples/task-list/tasks.json\"",
            &format!("{:?}", data_file.to_str().unwrap()),
        );

        let config = RhaiConfig {
            features: RhaiFeatures {
                file_io: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = RhaiEngine::new(config);
        let ctx = Arc::new(MockContext::default());
        engine.register_context(ctx.clone());
        engine
            .load_script("handlers", &script)
            .expect("task-list handlers.rhai should compile");

        // Number of rows the table's `data` property currently holds.
        let table_len = |ctx: &MockContext| -> usize {
            match ctx.get_component_property("tasks_table", "data") {
                Some(PluginValue::Array(rows)) => rows.len(),
                _ => panic!("tasks_table data should be an array"),
            }
        };

        // on_load: no file yet → create an empty one and render an empty table.
        engine
            .call::<()>(
                "handlers",
                "on_load",
                ("app".to_string(), "load".to_string()),
            )
            .expect("on_load should run");
        assert!(data_file.exists(), "on_load should create tasks.json");
        assert_eq!(table_len(&ctx), 0);
        assert_eq!(
            ctx.get_component_property("task_count", "text"),
            Some(PluginValue::String("0 tasks".to_string()))
        );

        // Add a task via the modal: the inputs' `value` props stand in for typed
        // text (kept live by the input readback wiring in the real app).
        ctx.set_component_property(
            "new_task_input",
            "value",
            PluginValue::String("Write the report".to_string()),
        )
        .unwrap();
        ctx.set_component_property(
            "new_due_input",
            "value",
            PluginValue::String("2026-07-20".to_string()),
        )
        .unwrap();
        engine
            .call::<()>(
                "handlers",
                "submit_add",
                ("add_confirm".to_string(), "click".to_string()),
            )
            .expect("submit_add should run");

        // Persisted, rendered, fields cleared, modal closed.
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&data_file).unwrap()).unwrap();
        assert_eq!(
            parsed["tasks"][0]["label"],
            serde_json::json!("Write the report")
        );
        assert_eq!(parsed["tasks"][0]["done"], serde_json::json!(false));
        assert_eq!(table_len(&ctx), 1);
        assert_eq!(
            ctx.get_component_property("task_count", "text"),
            Some(PluginValue::String("1 task".to_string()))
        );
        assert_eq!(
            ctx.get_component_property("new_task_input", "value"),
            Some(PluginValue::String(String::new()))
        );
        assert_eq!(
            ctx.get_component_property("add_modal", "open"),
            Some(PluginValue::Bool(false))
        );

        // An empty description is ignored (no phantom row).
        ctx.set_component_property(
            "new_task_input",
            "value",
            PluginValue::String(String::new()),
        )
        .unwrap();
        engine
            .call::<()>(
                "handlers",
                "submit_add",
                ("add_confirm".to_string(), "click".to_string()),
            )
            .expect("submit_add should run");
        assert_eq!(table_len(&ctx), 1);

        // Add a second task so we can toggle/delete against a specific row.
        ctx.set_component_property(
            "new_task_input",
            "value",
            PluginValue::String("Buy groceries".to_string()),
        )
        .unwrap();
        engine
            .call::<()>(
                "handlers",
                "submit_add",
                ("add_confirm".to_string(), "click".to_string()),
            )
            .expect("submit_add should run");
        assert_eq!(table_len(&ctx), 2);

        // Toggle row 1 done (rows are 1-based in the "#" column).
        ctx.set_component_property("row_num", "value", PluginValue::String("1".to_string()))
            .unwrap();
        engine
            .call::<()>(
                "handlers",
                "toggle_done",
                ("toggle_button".to_string(), "click".to_string()),
            )
            .expect("toggle_done should run");
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&data_file).unwrap()).unwrap();
        assert_eq!(parsed["tasks"][0]["done"], serde_json::json!(true));
        assert_eq!(parsed["tasks"][1]["done"], serde_json::json!(false));

        // A non-numeric row number is a no-op, not a crash.
        ctx.set_component_property("row_num", "value", PluginValue::String("abc".to_string()))
            .unwrap();
        engine
            .call::<()>(
                "handlers",
                "toggle_done",
                ("toggle_button".to_string(), "click".to_string()),
            )
            .expect("toggle_done with bad input should not error");
        assert_eq!(table_len(&ctx), 2);

        // Delete row 1: only the second task should remain.
        ctx.set_component_property("row_num", "value", PluginValue::String("1".to_string()))
            .unwrap();
        engine
            .call::<()>(
                "handlers",
                "delete_task",
                ("delete_button".to_string(), "click".to_string()),
            )
            .expect("delete_task should run");
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&data_file).unwrap()).unwrap();
        assert_eq!(parsed["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(
            parsed["tasks"][0]["label"],
            serde_json::json!("Buy groceries")
        );
        assert_eq!(table_len(&ctx), 1);
    }

    #[cfg(feature = "pkg-env")]
    #[cfg(feature = "pkg-sci")]
    #[cfg(feature = "pkg-process")]
    #[test]
    fn test_dev_dashboard_handlers_compile() {
        // Verify the dev-dashboard example's handler script compiles with
        // all features enabled. The script uses rhai-env (env), rhai-process
        // (cmd), rhai-sci (mean/std/median/min/max), rhai-chrono
        // (datetime_local/datetime_utc), and the built-in http_get.
        let script = include_str!("../../../examples/dev-dashboard/scripts/handlers.rhai");
        let config = RhaiConfig {
            features: RhaiFeatures {
                file_io: true,
                system: true,
                science: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = RhaiEngine::new(config);
        engine
            .load_script("handlers", script)
            .expect("dev-dashboard handlers.rhai should compile");
    }

    #[cfg(feature = "pkg-sci")]
    #[test]
    fn test_dev_dashboard_stats_runtime() {
        // Runtime guard for the dev-dashboard's shared-state pattern (which
        // had the same module-state bug as task-list): seed the sample store
        // via set_data, then call update_stats and confirm it reads them back
        // (get_data + json + helper-fn constants) and computes the rhai-sci
        // stats. No network or subprocess — update_stats touches neither.
        let script = include_str!("../../../examples/dev-dashboard/scripts/handlers.rhai");
        let config = RhaiConfig {
            features: RhaiFeatures {
                science: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = RhaiEngine::new(config);
        let ctx = Arc::new(MockContext::default());
        engine.register_context(ctx.clone());
        engine.load_script("handlers", script).unwrap();

        ctx.set_data(
            "dev_dashboard.samples",
            PluginValue::String("[10.0, 20.0, 30.0]".to_string()),
        )
        .unwrap();

        engine
            .call::<()>("handlers", "update_stats", ())
            .expect("update_stats should run");

        assert_eq!(
            ctx.get_component_property("stats_samples", "text"),
            Some(PluginValue::String("Samples: 3".to_string()))
        );
        // mean([10,20,30]) == 20 → "20 ms"
        match ctx.get_component_property("stats_mean", "text") {
            Some(PluginValue::String(s)) => assert!(s.contains("20"), "mean text: {s}"),
            other => panic!("unexpected stats_mean: {other:?}"),
        }

        // reset_stats clears the store and zeroes the display.
        engine
            .call::<()>(
                "handlers",
                "reset_stats",
                ("btn".to_string(), String::new()),
            )
            .expect("reset_stats should run");
        assert_eq!(
            ctx.get_component_property("stats_samples", "text"),
            Some(PluginValue::String("Samples: 0".to_string()))
        );
    }

    #[cfg(feature = "pkg-env")]
    #[cfg(feature = "pkg-process")]
    #[cfg(feature = "pkg-sci")]
    #[test]
    fn test_dev_dashboard_refresh_all_runtime() {
        // End-to-end guard for the exact failure the user hit: `refresh_all`
        // fans out to refresh_env (rhai-env `env`), refresh_sys (rhai-process
        // `cmd`), refresh_clock (rhai-chrono), and update_stats (rhai-sci).
        // With all packages registered (system + science features) every
        // function must resolve — no "Function not found: env".
        let script = include_str!("../../../examples/dev-dashboard/scripts/handlers.rhai");
        let config = RhaiConfig {
            features: RhaiFeatures {
                system: true,
                science: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = RhaiEngine::new(config);
        let ctx = Arc::new(MockContext::default());
        engine.register_context(ctx.clone());
        engine.load_script("handlers", script).unwrap();

        engine
            .call::<()>(
                "handlers",
                "refresh_all",
                ("btn".to_string(), String::new()),
            )
            .expect("refresh_all should run with all packages registered");

        // refresh_clock (chrono) and refresh_env (env) wrote their panels, and
        // update_stats (sci) ran against the empty sample set.
        assert!(
            ctx.get_component_property("clock", "text").is_some(),
            "refresh_clock should set the clock"
        );
        assert!(
            ctx.get_component_property("env_path", "text").is_some(),
            "refresh_env should set env_path"
        );
        // refresh_sys (rhai-process) must populate the System Info panel with a
        // real `uname` result, not stay at the placeholder or an error string.
        match ctx.get_component_property("sys_os", "text") {
            Some(PluginValue::String(s)) => {
                assert!(
                    !s.is_empty() && !s.starts_with("uname failed"),
                    "sys_os should hold a real OS name, got {s:?}"
                );
            }
            other => panic!("sys_os not set by refresh_sys: {other:?}"),
        }
        assert_eq!(
            ctx.get_component_property("stats_samples", "text"),
            Some(PluginValue::String("Samples: 0".to_string()))
        );
    }

    #[cfg(feature = "pkg-env")]
    #[cfg(not(feature = "pkg-process"))]
    #[test]
    fn test_dev_dashboard_refresh_all_degrades_without_process() {
        // With rhai-process NOT compiled in (the default build), `cmd` does not
        // exist. refresh_sys wraps each command in try/catch, so refresh_all
        // must still complete: clock/env/stats update, and the System Info panel
        // shows an "unavailable" message rather than aborting the handler.
        let script = include_str!("../../../examples/dev-dashboard/scripts/handlers.rhai");
        let config = RhaiConfig {
            features: RhaiFeatures {
                system: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut engine = RhaiEngine::new(config);
        let ctx = Arc::new(MockContext::default());
        engine.register_context(ctx.clone());
        engine.load_script("handlers", script).unwrap();

        engine
            .call::<()>(
                "handlers",
                "refresh_all",
                ("btn".to_string(), String::new()),
            )
            .expect("refresh_all must not abort when pkg-process is absent");

        // Other panels still updated.
        assert!(ctx.get_component_property("clock", "text").is_some());
        assert!(ctx.get_component_property("env_path", "text").is_some());
        assert_eq!(
            ctx.get_component_property("stats_samples", "text"),
            Some(PluginValue::String("Samples: 0".to_string()))
        );
        // System Info degraded gracefully.
        match ctx.get_component_property("sys_os", "text") {
            Some(PluginValue::String(s)) => assert!(
                s.contains("unavailable"),
                "sys_os should show an unavailable message, got {s:?}"
            ),
            other => panic!("sys_os not set: {other:?}"),
        }
    }
}
