use sqlx::{Pool, Postgres};
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use helios_crdt::Document;

/// A snapshot of the document at a point in time.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub document_json: String,
}

impl Snapshot {
    /// Save a snapshot to the database.
    pub async fn save(pool: &Pool<Postgres>, doc: &Document) -> sqlx::Result<i64> {
        let json = serde_json::to_string(doc).expect("document serializable");
        let rec = sqlx::query!(
            "INSERT INTO snapshots (timestamp, document_json) VALUES (now(), $1) RETURNING id",
            json
        )
        .fetch_one(pool)
        .await?;
        Ok(rec.id)
    }

    /// Load a snapshot by id.
    pub async fn load(pool: &Pool<Postgres>, id: i64) -> sqlx::Result<Self> {
        let rec = sqlx::query_as!(Self, "SELECT id, timestamp as \"timestamp!: DateTime<Utc>\", document_json FROM snapshots WHERE id = $1", id)
            .fetch_one(pool)
            .await?;
        Ok(rec)
    }
}

/// Simple API helpers that can be wired into the Axum router.
pub async fn create_snapshot_handler(state: axum::extract::State<crate::AppState>) -> impl axum::response::IntoResponse {
    if let Some(pool) = &state.db {
        let _ = Snapshot::save(pool, &state.doc).await;
        (axum::http::StatusCode::CREATED, "snapshot created")
    } else {
        (axum::http::StatusCode::BAD_REQUEST, "persistence disabled")
    }
}

pub async fn list_snapshots_handler(state: axum::extract::State<crate::AppState>) -> impl axum::response::IntoResponse {
    if let Some(pool) = &state.db {
        let rows = sqlx::query_as!(Snapshot, "SELECT id, timestamp as \"timestamp!: DateTime<Utc>\", document_json FROM snapshots ORDER BY timestamp DESC")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        (axum::http::StatusCode::OK, axum::Json(rows))
    } else {
        (axum::http::StatusCode::BAD_REQUEST, "persistence disabled")
    }
}
