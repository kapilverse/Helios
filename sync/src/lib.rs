use helios_crdt::{Op, OpId};
pub use helios_presence::CursorPosition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub document_id: String,
    pub last_seen_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub ops: Vec<(u64, Op)>,
    pub current_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join {
        document_id: String,
        name: String,
        color: String,
    },
    Op {
        op: Op,
    },
    Sync {
        request: SyncRequest,
    },
    Presence {
        cursor: Option<OpId>,
        selection_start: Option<OpId>,
        selection_end: Option<OpId>,
        viewport_top: Option<OpId>,
        viewport_bottom: Option<OpId>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Op { op: Op, seq: u64 },
    Sync { response: SyncResponse },
    Presence { peers: Vec<CursorPosition> },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct SyncState {
    pub last_seen_seq: u64,
}

impl SyncState {
    pub fn new() -> Self {
        Self { last_seen_seq: 0 }
    }

    pub fn needs_sync(&self, server_seq: u64) -> bool {
        self.last_seen_seq < server_seq
    }

    pub fn update(&mut self, seq: u64) {
        self.last_seen_seq = seq;
    }
}

impl Default for SyncState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state() {
        let mut state = SyncState::new();
        assert!(!state.needs_sync(0));
        assert!(state.needs_sync(1));

        state.update(5);
        assert!(!state.needs_sync(5));
        assert!(state.needs_sync(6));
    }
}
