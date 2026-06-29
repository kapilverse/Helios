use helios_crdt::{Document, Op};
use helios_network::{app, AppState};
use sqlx::{postgres::PgPoolOptions, Row};
use std::{path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let mut initial_doc = Document::new();
    let db = match std::env::var("DATABASE_URL") {
        Ok(db_url) => {
            let pool = PgPoolOptions::new().max_connections(5).connect(&db_url).await?;
            sqlx::migrate!("./migrations").run(&pool).await?;
            tracing::info!("Connected to Neon Postgres and applied migrations");

            let records = sqlx::query("SELECT op_data FROM operations ORDER BY seq ASC")
                .fetch_all(&pool)
                .await?;

            for record in records {
                let op_data: serde_json::Value = record.get("op_data");
                if let Ok(op) = serde_json::from_value::<Op>(op_data) {
                    initial_doc.apply(op);
                }
            }
            tracing::info!("Reconstructed document with {} ops", initial_doc.op_log.len());
            Some(pool)
        }
        Err(_) => {
            tracing::warn!("DATABASE_URL not set, running without persistence");
            None
        }
    };

    let state = Arc::new(AppState::new(db, initial_doc));

    // Try to find static files directory
    let static_dir = ["frontend/dist", "static"]
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
