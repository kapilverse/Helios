use helios_network::{app, AppState};
use std::{path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());

    // Try to find static files directory
    let static_dir = ["static", "frontend/dist"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists());

    if let Some(ref dir) = static_dir {
        tracing::info!("Serving static files from: {:?}", dir);
    } else {
        tracing::warn!("No static directory found, frontend won't be served");
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Helios server listening on http://0.0.0.0:3000");

    axum::serve(listener, app(state, static_dir)).await?;
    Ok(())
}
