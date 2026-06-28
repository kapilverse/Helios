use helios_crdt::OpId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEntry {
    pub peer_id: Uuid,
    pub name: String,
    pub color: String,
    pub cursor: Option<OpId>,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct PresenceMap {
    peers: HashMap<Uuid, PresenceEntry>,
    heartbeat_timeout_ms: u64,
}

impl PresenceMap {
    pub fn new(heartbeat_timeout_ms: u64) -> Self {
        Self {
            peers: HashMap::new(),
            heartbeat_timeout_ms,
        }
    }

    pub fn update(
        &mut self,
        peer_id: Uuid,
        name: String,
        color: String,
        cursor: Option<OpId>,
        now: u64,
    ) {
        self.peers.insert(
            peer_id,
            PresenceEntry {
                peer_id,
                name,
                color,
                cursor,
                last_seen: now,
            },
        );
    }

    pub fn remove(&mut self, peer_id: &Uuid) {
        self.peers.remove(peer_id);
    }

    pub fn cleanup_stale(&mut self, now: u64) {
        self.peers
            .retain(|_, entry| now.saturating_sub(entry.last_seen) < self.heartbeat_timeout_ms);
    }

    pub fn get_all(&self) -> Vec<&PresenceEntry> {
        self.peers.values().collect()
    }

    pub fn get(&self, peer_id: &Uuid) -> Option<&PresenceEntry> {
        self.peers.get(peer_id)
    }
}

impl Default for PresenceMap {
    fn default() -> Self {
        Self::new(5000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_update_and_get() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(peer, "Alice".to_string(), "#ff0000".to_string(), None, 1000);

        let entry = map.get(&peer).unwrap();
        assert_eq!(entry.name, "Alice");
    }

    #[test]
    fn test_stale_removal() {
        let mut map = PresenceMap::new(5000);
        let peer1 = Uuid::new_v4();
        let peer2 = Uuid::new_v4();

        map.update(peer1, "A".into(), "#f00".into(), None, 0);
        map.update(peer2, "B".into(), "#0f0".into(), None, 4000);

        map.cleanup_stale(5000);

        assert!(map.get(&peer1).is_none());
        assert!(map.get(&peer2).is_some());
    }
}
