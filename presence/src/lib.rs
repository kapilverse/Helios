use helios_crdt::OpId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

const DEFAULT_HEARTBEAT_MS: u64 = 5000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub op_id: Option<OpId>,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct PresenceEntry {
    pub peer_id: Uuid,
    pub name: String,
    pub color: String,
    pub cursor: Option<OpId>,
    pub selection_start: Option<OpId>,
    pub selection_end: Option<OpId>,
    pub viewport_top: Option<OpId>,
    pub viewport_bottom: Option<OpId>,
    pub last_seen: u64,
}

impl PresenceEntry {
    pub fn snapshot(&self) -> CursorPosition {
        CursorPosition {
            op_id: self.cursor,
            name: self.name.clone(),
            color: self.color.clone(),
        }
    }
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
        let entry = self.peers.entry(peer_id).or_insert_with(|| PresenceEntry {
            peer_id,
            name: name.clone(),
            color: color.clone(),
            cursor,
            selection_start: None,
            selection_end: None,
            viewport_top: None,
            viewport_bottom: None,
            last_seen: now,
        });
        entry.name = name;
        entry.color = color;
        entry.cursor = cursor;
        entry.last_seen = now;
    }

    pub fn update_selection(
        &mut self,
        peer_id: &Uuid,
        start: Option<OpId>,
        end: Option<OpId>,
        now: u64,
    ) {
        if let Some(entry) = self.peers.get_mut(peer_id) {
            entry.selection_start = start;
            entry.selection_end = end;
            entry.last_seen = now;
        }
    }

    pub fn update_viewport(
        &mut self,
        peer_id: &Uuid,
        top: Option<OpId>,
        bottom: Option<OpId>,
        now: u64,
    ) {
        if let Some(entry) = self.peers.get_mut(peer_id) {
            entry.viewport_top = top;
            entry.viewport_bottom = bottom;
            entry.last_seen = now;
        }
    }

    pub fn heartbeat(&mut self, peer_id: &Uuid, now: u64) {
        if let Some(entry) = self.peers.get_mut(peer_id) {
            entry.last_seen = now;
        }
    }

    pub fn remove(&mut self, peer_id: &Uuid) {
        self.peers.remove(peer_id);
    }

    pub fn cleanup_stale(&mut self, now: u64) -> Vec<Uuid> {
        let mut removed = Vec::new();
        self.peers.retain(|id, entry| {
            if now.saturating_sub(entry.last_seen) >= self.heartbeat_timeout_ms {
                removed.push(*id);
                false
            } else {
                true
            }
        });
        removed
    }

    pub fn get_all(&self) -> Vec<&PresenceEntry> {
        self.peers.values().collect()
    }

    pub fn get_all_snapshots(&self) -> Vec<CursorPosition> {
        self.peers.values().map(|e| e.snapshot()).collect()
    }

    pub fn get(&self, peer_id: &Uuid) -> Option<&PresenceEntry> {
        self.peers.get(peer_id)
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    pub fn heartbeat_timeout_ms(&self) -> u64 {
        self.heartbeat_timeout_ms
    }
}

impl Default for PresenceMap {
    fn default() -> Self {
        Self::new(DEFAULT_HEARTBEAT_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_update_and_get() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(peer, "Alice".into(), "#ff0000".into(), None, 1000);

        let entry = map.get(&peer).unwrap();
        assert_eq!(entry.name, "Alice");
        assert_eq!(entry.color, "#ff0000");
    }

    #[test]
    fn test_stale_removal() {
        let mut map = PresenceMap::new(5000);
        let peer1 = Uuid::new_v4();
        let peer2 = Uuid::new_v4();

        map.update(peer1, "A".into(), "#f00".into(), None, 0);
        map.update(peer2, "B".into(), "#0f0".into(), None, 4000);

        let removed = map.cleanup_stale(5000);
        assert_eq!(removed.len(), 1);
        assert!(removed.contains(&peer1));
        assert!(map.get(&peer1).is_none());
        assert!(map.get(&peer2).is_some());
    }

    #[test]
    fn test_heartbeat_extends_lifetime() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(peer, "A".into(), "#f00".into(), None, 0);
        assert!(map.cleanup_stale(4000).is_empty());

        map.heartbeat(&peer, 3000);
        assert!(map.cleanup_stale(6000).is_empty());
    }

    #[test]
    fn test_selection_tracking() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(
            peer,
            "A".into(),
            "#f00".into(),
            Some(OpId::new(Uuid::nil(), 1)),
            0,
        );
        map.update_selection(
            &peer,
            Some(OpId::new(Uuid::nil(), 1)),
            Some(OpId::new(Uuid::nil(), 5)),
            100,
        );

        let entry = map.get(&peer).unwrap();
        assert_eq!(entry.selection_start, Some(OpId::new(Uuid::nil(), 1)));
        assert_eq!(entry.selection_end, Some(OpId::new(Uuid::nil(), 5)));
    }

    #[test]
    fn test_viewport_tracking() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(peer, "A".into(), "#f00".into(), None, 0);
        map.update_viewport(
            &peer,
            Some(OpId::new(Uuid::nil(), 10)),
            Some(OpId::new(Uuid::nil(), 50)),
            100,
        );

        let entry = map.get(&peer).unwrap();
        assert_eq!(entry.viewport_top, Some(OpId::new(Uuid::nil(), 10)));
        assert_eq!(entry.viewport_bottom, Some(OpId::new(Uuid::nil(), 50)));
    }

    #[test]
    fn test_snapshot() {
        let mut map = PresenceMap::new(5000);
        let peer = Uuid::new_v4();

        map.update(
            peer,
            "Alice".into(),
            "#ff0000".into(),
            Some(OpId::new(Uuid::nil(), 5)),
            0,
        );

        let snapshots = map.get_all_snapshots();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].name, "Alice");
        assert_eq!(snapshots[0].op_id, Some(OpId::new(Uuid::nil(), 5)));
    }
}
