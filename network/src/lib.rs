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
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct AppState {
    pub document: RwLock<Document>,
    pub reconciler: OtReconciler,
    pub presence: RwLock<PresenceMap>,
    pub sync_states: RwLock<HashMap<Uuid, SyncState>>,
    pub op_seq: RwLock<u64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            document: RwLock::new(Document::new()),
            reconciler: OtReconciler::new(),
            presence: RwLock::new(PresenceMap::default()),
            sync_states: RwLock::new(HashMap::new()),
            op_seq: RwLock::new(0),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let peer_id = Uuid::new_v4();
    let sync_state = SyncState::new();

    {
        let mut sync_states = state.sync_states.write().await;
        sync_states.insert(peer_id, sync_state.clone());
    }

    state.presence.write().await.update(
        peer_id,
        format!("User-{}", &peer_id.to_string()[..8]),
        "#3b82f6".to_string(),
        None,
        0,
    );

    let (mut sender, mut receiver) = socket.split();

    // Send welcome
    let welcome = serde_json::to_string(&ServerMessage::Sync {
        response: helios_sync::SyncResponse {
            ops: vec![],
            current_seq: *state.op_seq.read().await,
        },
    })
    .unwrap();
    let _ = sender.send(Message::Text(welcome.into())).await;

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text.to_string(),
            Ok(Message::Close(_)) => break,
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match client_msg {
            ClientMessage::Op { op } => {
                let mut doc = state.document.write().await;
                let mut seq = state.op_seq.write().await;
                *seq += 1;
                let current_seq = *seq;

                // Apply with reconciler
                let existing_ops = doc.op_log.ops().to_vec();
                let mut corrected = op.clone();
                if let Some(last) = existing_ops.last() {
                    corrected = state.reconciler.transform(&op, last);
                }
                doc.apply(corrected.clone());

                let response = serde_json::to_string(&ServerMessage::Op {
                    op: corrected,
                    seq: current_seq,
                })
                .unwrap();
                let _ = sender.send(Message::Text(response.into())).await;
            }
            ClientMessage::Sync { request } => {
                let doc = state.document.read().await;
                let seq = *state.op_seq.read().await;
                let ops: Vec<(u64, Op)> = doc
                    .op_log
                    .ops()
                    .iter()
                    .enumerate()
                    .skip(request.last_seen_seq as usize)
                    .map(|(i, op)| (i as u64, op.clone()))
                    .collect();

                let response = serde_json::to_string(&ServerMessage::Sync {
                    response: helios_sync::SyncResponse {
                        ops,
                        current_seq: seq,
                    },
                })
                .unwrap();
                let _ = sender.send(Message::Text(response.into())).await;
            }
            ClientMessage::Join { .. } => {}
            ClientMessage::Presence { cursor } => {
                let mut presence = state.presence.write().await;
                presence.update(
                    peer_id,
                    format!("User-{}", &peer_id.to_string()[..8]),
                    "#3b82f6".to_string(),
                    cursor.and_then(|c| c.op_id),
                    0,
                );
            }
        }
    }

    state.presence.write().await.remove(&peer_id);
    state.sync_states.write().await.remove(&peer_id);
}
