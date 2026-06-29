use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use helios_crdt::{Document, Op};
use helios_ot_reconciler::OtReconciler;
use helios_presence::PresenceMap;
use helios_sync::{ClientMessage, ServerMessage, SyncState};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub document: RwLock<Document>,
    pub reconciler: OtReconciler,
    pub presence: RwLock<PresenceMap>,
    pub sync_states: RwLock<HashMap<Uuid, SyncState>>,
    pub op_seq: RwLock<u64>,
    pub peers: RwLock<HashMap<Uuid, tokio::sync::mpsc::Sender<String>>>,
}

impl AppState {
    pub fn new(db: sqlx::PgPool, initial_document: Document) -> Self {
        let op_seq_val = initial_document.op_log.len() as u64;
        Self {
            db,
            document: RwLock::new(initial_document),
            reconciler: OtReconciler::new(),
            presence: RwLock::new(PresenceMap::default()),
            sync_states: RwLock::new(HashMap::new()),
            op_seq: RwLock::new(op_seq_val),
            peers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn broadcast(&self, exclude: Option<Uuid>, msg: &str) {
        let peers = self.peers.read().await;
        for (id, tx) in peers.iter() {
            if Some(*id) != exclude {
                let _ = tx.send(msg.to_string()).await;
            }
        }
    }

    async fn broadcast_presence(&self) {
        let snapshots = self.presence.read().await.get_all_snapshots();
        let msg = serde_json::to_string(&ServerMessage::Presence { peers: snapshots }).unwrap();
        self.broadcast(None, &msg).await;
    }
}

pub fn app(state: Arc<AppState>, static_dir: Option<PathBuf>) -> Router {
    // Spawn heartbeat cleanup task
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        let interval = heartbeat_state.presence.read().await.heartbeat_timeout_ms();
        let mut timer = tokio::time::interval(tokio::time::Duration::from_millis(interval / 2));
        timer.tick().await;
        loop {
            timer.tick().await;
            let now = timestamp_ms();
            let removed = {
                let mut presence = heartbeat_state.presence.write().await;
                presence.cleanup_stale(now)
            };
            if !removed.is_empty() {
                heartbeat_state.broadcast_presence().await;
            }
        }
    });

    let mut router = Router::new()
        .route("/ws", get(ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);

    if let Some(dir) = static_dir {
        router = router.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true));
    }

    router
}

fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let peer_id = Uuid::new_v4();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);
    state.peers.write().await.insert(peer_id, tx);

    state
        .sync_states
        .write()
        .await
        .insert(peer_id, SyncState::new());

    state.presence.write().await.update(
        peer_id,
        format!("User-{}", &peer_id.to_string()[..8]),
        "#3b82f6".to_string(),
        None,
        timestamp_ms(),
    );

    let current_seq = *state.op_seq.read().await;
    let welcome = serde_json::to_string(&ServerMessage::Sync {
        response: helios_sync::SyncResponse {
            ops: vec![],
            current_seq,
        },
    })
    .unwrap();
    let _ = ws_sender.send(Message::Text(welcome)).await;

    // Broadcast updated presence (new user joined)
    state.broadcast_presence().await;

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = ws_receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text.to_string(),
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match client_msg {
            ClientMessage::Join { document_id: _ } => {}

            ClientMessage::Op { op } => {
                let mut doc = state.document.write().await;
                let mut seq = state.op_seq.write().await;
                *seq += 1;
                let current_seq = *seq;

                let last_op = doc.op_log.ops().last().cloned();
                let corrected = state.reconciler.reconcile(&mut doc, op, last_op.as_ref());
                drop(doc);
                drop(seq);

                for corrected_op in corrected {
                    // Save to PostgreSQL
                    if let Ok(op_json) = serde_json::to_value(&corrected_op) {
                        let _ = sqlx::query!(
                            "INSERT INTO operations (op_data) VALUES ($1)",
                            op_json
                        )
                        .execute(&state.db)
                        .await;
                    }

                    let server_msg = serde_json::to_string(&ServerMessage::Op {
                        op: corrected_op,
                        seq: current_seq,
                    })
                    .unwrap();
                    state.broadcast(None, &server_msg).await;
                }
            }

            ClientMessage::Sync { request } => {
                let doc = state.document.read().await;
                let current_seq = *state.op_seq.read().await;
                let ops: Vec<(u64, Op)> = doc
                    .op_log
                    .ops()
                    .iter()
                    .enumerate()
                    .skip(request.last_seen_seq as usize)
                    .map(|(i, op)| (i as u64, op.clone()))
                    .collect();

                let response = serde_json::to_string(&ServerMessage::Sync {
                    response: helios_sync::SyncResponse { ops, current_seq },
                })
                .unwrap();
                let _ = state
                    .peers
                    .read()
                    .await
                    .get(&peer_id)
                    .map(|tx| tx.try_send(response));
            }

            ClientMessage::Presence {
                cursor,
                selection_start,
                selection_end,
                viewport_top,
                viewport_bottom,
            } => {
                let now = timestamp_ms();
                {
                    let mut presence = state.presence.write().await;
                    presence.update(
                        peer_id,
                        format!("User-{}", &peer_id.to_string()[..8]),
                        "#3b82f6".to_string(),
                        cursor,
                        now,
                    );
                    presence.update_selection(&peer_id, selection_start, selection_end, now);
                    presence.update_viewport(&peer_id, viewport_top, viewport_bottom, now);
                }

                state.broadcast_presence().await;
            }
        }
    }

    state.peers.write().await.remove(&peer_id);
    state.presence.write().await.remove(&peer_id);
    state.sync_states.write().await.remove(&peer_id);

    state.broadcast_presence().await;
}
