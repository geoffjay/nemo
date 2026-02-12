//! Host-side implementation of WIT-imported functions.
//!
//! `HostState` lives inside each plugin's `Store` and delegates WIT host-api
//! calls to the shared `Arc<dyn PluginContext>`.

use crate::convert;
use crate::nemo::plugin::host_api;
use crate::nemo::plugin::types::{self, LogLevel, PluginValue};
use nemo_plugin_api::PluginContext;
use std::sync::Arc;
use wasmtime_wasi::{WasiCtx, WasiView};

/// Per-plugin state stored in a `wasmtime::Store`.
pub struct HostState {
    pub(crate) context: Arc<dyn PluginContext>,
    pub(crate) wasi_ctx: WasiCtx,
    pub(crate) wasi_table: wasmtime::component::ResourceTable,
}

impl HostState {
    pub fn new(context: Arc<dyn PluginContext>, wasi_ctx: WasiCtx) -> Self {
        Self {
            context,
            wasi_ctx,
            wasi_table: wasmtime::component::ResourceTable::new(),
        }
    }
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.wasi_table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl types::Host for HostState {}

impl host_api::Host for HostState {
    fn get_data(&mut self, path: String) -> Option<PluginValue> {
        self.context.get_data(&path).map(|v| convert::to_wit(&v))
    }

    fn set_data(&mut self, path: String, value: PluginValue) -> Result<(), String> {
        let native = convert::from_wit(&value);
        self.context
            .set_data(&path, native)
            .map_err(|e| e.to_string())
    }

    fn emit_event(&mut self, event_type: String, payload: PluginValue) {
        let native = convert::from_wit(&payload);
        self.context.emit_event(&event_type, native);
    }

    fn get_config(&mut self, path: String) -> Option<PluginValue> {
        self.context.get_config(&path).map(|v| convert::to_wit(&v))
    }

    fn log(&mut self, level: LogLevel, message: String) {
        let native = convert::log_level_from_wit(level);
        self.context.log(native, &message);
    }

    fn get_component_property(&mut self, id: String, prop: String) -> Option<PluginValue> {
        self.context
            .get_component_property(&id, &prop)
            .map(|v| convert::to_wit(&v))
    }

    fn set_component_property(
        &mut self,
        id: String,
        prop: String,
        value: PluginValue,
    ) -> Result<(), String> {
        let native = convert::from_wit(&value);
        self.context
            .set_component_property(&id, &prop, native)
            .map_err(|e| e.to_string())
    }
}
