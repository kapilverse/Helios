use axum::{extract::State, Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub wasm_path: String,
}

impl crate::AppState {
    pub fn register_plugin(&self, plugin: PluginInfo) {
        let mut plugins = self.plugins.lock().unwrap();
        plugins.insert(plugin.name.clone(), plugin);
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.lock().unwrap();
        plugins.values().cloned().collect()
    }
}

pub async fn add_plugin_handler(State(state): State<Arc<crate::AppState>>, Json(plugin): Json<PluginInfo>) -> impl IntoResponse {
    // TODO: verify admin rights
    state.register_plugin(plugin);
    (StatusCode::CREATED, "plugin registered")
}

pub async fn list_plugins_handler(State(state): State<Arc<crate::AppState>>) -> impl IntoResponse {
    let list = state.list_plugins();
    (StatusCode::OK, Json(list))
}
