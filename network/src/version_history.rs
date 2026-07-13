use axum::{extract::State, response::IntoResponse, Json};
use axum::http::StatusCode;
use sqlx::{Pool, Postgres};
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use helios_crdt::Document;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub document_json: String,
}

impl Snapshot {
    pub async fn save(pool: &Pool<Postgres>, doc: &Document) -> sqlx::Result<i64> {
        let json = serde_json::to_string(doc).expect("document serializable");
        let rec = sqlx::query!("INSERT INTO snapshots (timestamp, document_json) VALUES (now(), $1) RETURNING id", json)
            .fetch_one(pool)
            .await?;
        Ok(rec.id)
    }

    pub async fn load(pool: &Pool<Postgres>, id: i64) -> sqlx::Result<Self> {
        let rec = sqlx::query_as!(Self, "SELECT id, timestamp as \"timestamp!: DateTime<Utc>\", document_json FROM snapshots WHERE id = $1", id)
            .fetch_one(pool)
            .await?;
        Ok(rec)
    }
}

pub async fn create_snapshot_handler(State(state): State<Arc<crate::AppState>>) -> impl IntoResponse {
    if let Some(pool) = &state.db {
        let rooms = state.rooms.read().await;
        if let Some(room) = rooms.get("default") {
            let _ = Snapshot::save(pool, &room.document).await;
            (StatusCode::CREATED, "snapshot created")
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, "default room missing")
        }
    } else {
        (StatusCode::BAD_REQUEST, "persistence disabled")
    }
}

pub async fn list_snapshots_handler(State(state): State<Arc<crate::AppState>>) -> impl IntoResponse {
    if let Some(pool) = &state.db {
        let rows = sqlx::query_as!(Snapshot, "SELECT id, timestamp as \"timestamp!: DateTime<Utc>\", document_json FROM snapshots ORDER BY timestamp DESC")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        (StatusCode::OK, Json(rows))
    } else {
        (StatusCode::BAD_REQUEST, "persistence disabled")
    }
}
