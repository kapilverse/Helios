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

    let port = std::env::var("HELIOS_PORT").unwrap_or_else(|_| "5174".to_string());
    let bind_addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Helios backend listening on http://{}", bind_addr);

    axum::serve(listener, app(state, None::<PathBuf>)).await?;
    Ok(())
}
