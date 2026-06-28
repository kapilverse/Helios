use helios_network::{app, AppState};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Helios server listening on 127.0.0.1:3000");

    axum::serve(listener, app(state)).await?;
    Ok(())
}
