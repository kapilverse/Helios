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
use helios_sync::{ClientMessage, CursorPosition, ServerMessage, SyncState};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct AppState {
    pub document: RwLock<Document>,
    pub reconciler: OtReconciler,
    pub presence: RwLock<PresenceMap>,
    pub sync_states: RwLock<HashMap<Uuid, SyncState>>,
    pub op_seq: RwLock<u64>,
    pub peers: RwLock<HashMap<Uuid, tokio::sync::mpsc::Sender<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            document: RwLock::new(Document::new()),
            reconciler: OtReconciler::new(),
            presence: RwLock::new(PresenceMap::default()),
            sync_states: RwLock::new(HashMap::new()),
            op_seq: RwLock::new(0),
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
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for this peer
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);
    state.peers.write().await.insert(peer_id, tx);

    // Initialize sync state
    state.sync_states.write().await.insert(peer_id, SyncState::new());

    // Add to presence
    state.presence.write().await.update(
        peer_id,
        format!("User-{}", &peer_id.to_string()[..8]),
        "#3b82f6".to_string(),
        None,
        0,
    );

    // Send welcome with current seq
    let current_seq = *state.op_seq.read().await;
    let welcome = serde_json::to_string(&ServerMessage::Sync {
        response: helios_sync::SyncResponse {
            ops: vec![],
            current_seq,
        },
    }).unwrap();
    let _ = ws_sender.send(Message::Text(welcome.into())).await;

    // Spawn writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
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
                    let server_msg = serde_json::to_string(&ServerMessage::Op {
                        op: corrected_op,
                        seq: current_seq,
                    }).unwrap();
                    state.broadcast(Some(peer_id), &server_msg).await;
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
                    response: helios_sync::SyncResponse {
                        ops,
                        current_seq,
                    },
                }).unwrap();
                let _ = state.peers.read().await.get(&peer_id)
                    .map(|tx| tx.try_send(response));
            }

            ClientMessage::Presence { cursor } => {
                let mut presence = state.presence.write().await;
                presence.update(
                    peer_id,
                    format!("User-{}", &peer_id.to_string()[..8]),
                    "#3b82f6".to_string(),
                    cursor.and_then(|c| c.op_id),
                    0,
                );

                let peers = state.presence.read().await;
                let all_peers: Vec<CursorPosition> = peers
                    .get_all()
                    .iter()
                    .map(|p| CursorPosition {
                        op_id: p.cursor,
                        name: p.name.clone(),
                        color: p.color.clone(),
                    })
                    .collect();
                drop(peers);

                let presence_msg = serde_json::to_string(&ServerMessage::Presence {
                    peers: all_peers,
                }).unwrap();
                state.broadcast(None, &presence_msg).await;
            }
        }
    }

    // Cleanup on disconnect
    state.peers.write().await.remove(&peer_id);
    state.presence.write().await.remove(&peer_id);
    state.sync_states.write().await.remove(&peer_id);

    // Broadcast presence update (user left)
    let peers = state.presence.read().await;
    let all_peers: Vec<CursorPosition> = peers
        .get_all()
        .iter()
        .map(|p| CursorPosition {
            op_id: p.cursor,
            name: p.name.clone(),
            color: p.color.clone(),
        })
        .collect();
    drop(peers);

    let presence_msg = serde_json::to_string(&ServerMessage::Presence {
        peers: all_peers,
    }).unwrap();
    state.broadcast(None, &presence_msg).await;
}
