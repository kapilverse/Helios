use std::sync::Arc;
use axum::{extract::State, Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::AppState;

/// Basic plugin descriptor – can be extended later.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub wasm_path: String,
}

/// In‑memory registry of available plugins.
impl AppState {
    pub fn register_plugin(&self, plugin: PluginInfo) {
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(plugin.name.clone(), plugin);
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.lock().unwrap();
        plugins.values().cloned().collect()
    }
}

/// Handler to add a plugin (admin only).
pub async fn add_plugin_handler(State(state): State<AppState>, Json(plugin): Json<PluginInfo>) -> impl IntoResponse {
    // TODO: verify admin rights via state.auth_checker
    state.register_plugin(plugin);
    (StatusCode::CREATED, "plugin registered")
}

/// Handler to list plugins.
pub async fn list_plugins_handler(State(state): State<AppState>) -> impl IntoResponse {
    let list = state.list_plugins();
    (StatusCode::OK, Json(list))
}
