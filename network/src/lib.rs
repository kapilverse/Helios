use axum::{
    routing::{get, post},
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use helios_crdt::Document;
use helios_ot_reconciler::OtReconciler;
use helios_presence::PresenceMap;
use helios_sync::{ClientMessage, ServerMessage, SyncState};
mod version_history;
mod plugin_manager;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use uuid::Uuid;

pub struct AppState {
    // existing fields
    pub db: Option<sqlx::PgPool>,
    pub rooms: RwLock<HashMap<String, DocumentRoom>>,
    pub peer_docs: RwLock<HashMap<Uuid, String>>,
    pub peers: RwLock<HashMap<Uuid, tokio::sync::mpsc::Sender<String>>>,
    pub reconciler: OtReconciler,
    // Plugin registry (in‑memory)
    pub plugins: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, plugin_manager::PluginInfo>>>,
}

pub struct DocumentRoom {
    pub document: Document,
    pub presence: PresenceMap,
    pub sync_states: HashMap<Uuid, SyncState>,
    pub op_seq: u64,
}

impl AppState {
    pub fn new(db: Option<sqlx::PgPool>, initial_document: Document) -> Self {
        let op_seq_val = initial_document.op_log.len() as u64;
        let mut rooms = HashMap::new();
        rooms.insert(
            "default".to_string(),
            DocumentRoom {
                document: initial_document,
                presence: PresenceMap::default(),
                sync_states: HashMap::new(),
                op_seq: op_seq_val,
            },
        );
        Self {
            db,
            rooms: RwLock::new(rooms),
            peer_docs: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            reconciler: OtReconciler::new(),
            plugins: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn broadcast(&self, document_id: &str, exclude: Option<Uuid>, msg: &str) {
        let peer_ids = {
            let rooms = self.rooms.read().await;
            rooms
                .get(document_id)
                .map(|room| {
                    room.presence
                        .get_all()
                        .into_iter()
                        .map(|entry| entry.peer_id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let peers = self.peers.read().await;
        for id in peer_ids {
            if Some(id) != exclude {
                if let Some(tx) = peers.get(&id) {
                    let _ = tx.send(msg.to_string()).await;
                }
            }
        }
    }

    async fn broadcast_presence(&self, document_id: &str) {
        let snapshots = {
            let rooms = self.rooms.read().await;
            rooms
                .get(document_id)
                .map(|room| room.presence.get_all_snapshots())
                .unwrap_or_default()
        };
        let msg = serde_json::to_string(&ServerMessage::Presence { peers: snapshots }).unwrap();
        self.broadcast(document_id, None, &msg).await;
    }
}

pub fn app(state: Arc<AppState>, static_dir: Option<PathBuf>) -> Router {
    // Spawn heartbeat cleanup task
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        let interval = {
            let rooms = heartbeat_state.rooms.read().await;
            rooms
                .values()
                .next()
                .map(|room| room.presence.heartbeat_timeout_ms())
                .unwrap_or(5000)
        };
        let mut timer = tokio::time::interval(tokio::time::Duration::from_millis(interval / 2));
        timer.tick().await;
        loop {
            timer.tick().await;
            let now = timestamp_ms();
            let room_ids: Vec<String> = {
                let rooms = heartbeat_state.rooms.read().await;
                rooms.keys().cloned().collect()
            };
            for document_id in room_ids {
                let removed = {
                    let mut rooms = heartbeat_state.rooms.write().await;
                    if let Some(room) = rooms.get_mut(&document_id) {
                        room.presence.cleanup_stale(now)
                    } else {
                        Vec::new()
                    }
                };
                if !removed.is_empty() {
                    heartbeat_state.broadcast_presence(&document_id).await;
                }
            }
        }
    });

    let mut router = Router::new()
        .route("/ws", get(ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        // Version history endpoints
        .route("/snapshot", get(crate::version_history::list_snapshots_handler))
        .route("/snapshot/create", get(crate::version_history::create_snapshot_handler))
        // Plugin management endpoints (admin only)
        .route("/plugins", get(crate::plugin_manager::list_plugins_handler))
        .route("/plugins/add", axum::routing::post(crate::plugin_manager::add_plugin_handler))
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
    let mut current_document = "default".to_string();
    {
        let mut rooms = state.rooms.write().await;
        let room = rooms
            .entry(current_document.clone())
            .or_insert_with(|| DocumentRoom {
                document: Document::new(),
                presence: PresenceMap::default(),
                sync_states: HashMap::new(),
                op_seq: 0,
            });
        room.sync_states.insert(peer_id, SyncState::new());
        room.presence.update(
            peer_id,
            format!("User-{}", &peer_id.to_string()[..8]),
            "#3b82f6".to_string(),
            None,
            timestamp_ms(),
        );
        let welcome = serde_json::to_string(&ServerMessage::Sync {
            response: helios_sync::SyncResponse {
                ops: vec![],
                current_seq: room.op_seq,
            },
        })
        .unwrap();
        let _ = ws_sender.send(Message::Text(welcome)).await;
    }

    state.broadcast_presence(&current_document).await;

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
            ClientMessage::Join {
                document_id,
                name,
                color,
            } => {
                let now = timestamp_ms();

                if document_id != current_document {
                    // Switch rooms
                    {
                        let mut rooms = state.rooms.write().await;
                        if let Some(room) = rooms.get_mut(&current_document) {
                            room.presence.remove(&peer_id);
                            room.sync_states.remove(&peer_id);
                        }
                        let room =
                            rooms
                                .entry(document_id.clone())
                                .or_insert_with(|| DocumentRoom {
                                    document: Document::new(),
                                    presence: PresenceMap::default(),
                                    sync_states: HashMap::new(),
                                    op_seq: 0,
                                });
                        room.sync_states.insert(peer_id, SyncState::new());
                        room.presence
                            .update(peer_id, name.clone(), color.clone(), None, now);

                        let current_seq = room.op_seq;
                        let welcome = serde_json::to_string(&ServerMessage::Sync {
                            response: helios_sync::SyncResponse {
                                ops: room
                                    .document
                                    .op_log
                                    .ops()
                                    .iter()
                                    .enumerate()
                                    .map(|(i, op)| (i as u64, op.clone()))
                                    .collect(),
                                current_seq,
                            },
                        })
                        .unwrap();
                        let _ = state
                            .peers
                            .read()
                            .await
                            .get(&peer_id)
                            .map(|tx| tx.try_send(welcome));
                    }

                    current_document = document_id;
                } else {
                    // Update name/color in current room
                    let mut rooms = state.rooms.write().await;
                    if let Some(room) = rooms.get_mut(&current_document) {
                        let cursor = room.presence.get(&peer_id).and_then(|p| p.cursor);
                        room.presence.update(peer_id, name, color, cursor, now);
                    }
                }

                state.broadcast_presence(&current_document).await;
            }

            ClientMessage::Op { op } => {
                let (corrected, current_seq) = {
                    let mut rooms = state.rooms.write().await;
                    let room =
                        rooms
                            .entry(current_document.clone())
                            .or_insert_with(|| DocumentRoom {
                                document: Document::new(),
                                presence: PresenceMap::default(),
                                sync_states: HashMap::new(),
                                op_seq: 0,
                            });
                    room.op_seq += 1;
                    let current_seq = room.op_seq;
                    let last_op = room.document.op_log.ops().last().cloned();
                    let corrected =
                        state
                            .reconciler
                            .reconcile(&mut room.document, op, last_op.as_ref());
                    (corrected, current_seq)
                };

                for corrected_op in corrected {
                    // Save to PostgreSQL in background to avoid blocking the WS loop
                    if let (Some(db), Ok(op_json)) =
                        (state.db.clone(), serde_json::to_value(&corrected_op))
                    {
                        tokio::spawn(async move {
                            let _ = sqlx::query("INSERT INTO operations (op_data) VALUES ($1)")
                                .bind(op_json)
                                .execute(&db)
                                .await;
                        });
                    }

                    let server_msg = serde_json::to_string(&ServerMessage::Op {
                        op: corrected_op,
                        seq: current_seq,
                    })
                    .unwrap();
                    state.broadcast(&current_document, None, &server_msg).await;
                }
            }

            ClientMessage::Sync { request } => {
                let rooms = state.rooms.read().await;
                let room = rooms.get(&current_document);
                let (ops, current_seq) = if let Some(room) = room {
                    (
                        room.document
                            .op_log
                            .ops()
                            .iter()
                            .enumerate()
                            .skip(request.last_seen_seq as usize)
                            .map(|(i, op)| (i as u64, op.clone()))
                            .collect(),
                        room.op_seq,
                    )
                } else {
                    (Vec::new(), 0)
                };

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
                    let mut rooms = state.rooms.write().await;
                    if let Some(room) = rooms.get_mut(&current_document) {
                        if let Some(entry) = room.presence.get(&peer_id) {
                            let name = entry.name.clone();
                            let color = entry.color.clone();
                            room.presence.update(peer_id, name, color, cursor, now);
                            room.presence.update_selection(
                                &peer_id,
                                selection_start,
                                selection_end,
                                now,
                            );
                            room.presence.update_viewport(
                                &peer_id,
                                viewport_top,
                                viewport_bottom,
                                now,
                            );
                        }
                    }
                }

                state.broadcast_presence(&current_document).await;
            }
        }
    }

    state.peers.write().await.remove(&peer_id);
    {
        let mut rooms = state.rooms.write().await;
        if let Some(room) = rooms.get_mut(&current_document) {
            room.presence.remove(&peer_id);
            room.sync_states.remove(&peer_id);
        }
    }
    state.broadcast_presence(&current_document).await;
}
