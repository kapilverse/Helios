use helios_crdt::{Document, Op};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

const SNAPSHOT_INTERVAL: usize = 1000;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("storage error: {0}")]
    Generic(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub seq: u64,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

pub struct OpStore {
    ops: Vec<Op>,
}

impl OpStore {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn append(&mut self, op: Op) -> u64 {
        let seq = self.ops.len() as u64;
        self.ops.push(op);
        seq
    }

    pub fn get_since(&self, seq: u64) -> Vec<(u64, Op)> {
        self.ops
            .iter()
            .enumerate()
            .skip(seq as usize)
            .map(|(i, op)| (i as u64, op.clone()))
            .collect()
    }

    pub fn get(&self, seq: u64) -> Option<&Op> {
        self.ops.get(seq as usize)
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Default for OpStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DocumentStore {
    op_store: OpStore,
    snapshots: Vec<Snapshot>,
    snapshot_interval: usize,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            op_store: OpStore::new(),
            snapshots: Vec::new(),
            snapshot_interval: SNAPSHOT_INTERVAL,
        }
    }

    pub fn with_snapshot_interval(interval: usize) -> Self {
        Self {
            op_store: OpStore::new(),
            snapshots: Vec::new(),
            snapshot_interval: interval,
        }
    }

    pub fn apply_op(&mut self, doc: &Document, op: Op) -> u64 {
        let seq = self.op_store.append(op);
        if self.should_snapshot() {
            self.create_snapshot(doc, seq);
        }
        seq
    }

    pub fn get_ops_since(&self, seq: u64) -> Vec<(u64, Op)> {
        self.op_store.get_since(seq)
    }

    pub fn load_document(&self) -> Document {
        let mut doc = Document::new();

        if let Some(snapshot) = self.latest_snapshot() {
            doc.sequence = helios_crdt::SequenceCrdt::from_string(&snapshot.content);
            let replay_ops = self.op_store.get_since(snapshot.seq + 1);
            for (_, op) in replay_ops {
                doc.apply(op);
            }
        } else {
            for (_, op) in self.op_store.get_since(0) {
                doc.apply(op);
            }
        }

        doc
    }

    pub fn create_snapshot(&mut self, doc: &Document, seq: u64) {
        let snapshot = Snapshot {
            seq,
            content: doc.content(),
            metadata: HashMap::new(),
        };
        self.snapshots.push(snapshot);
    }

    pub fn latest_snapshot(&self) -> Option<&Snapshot> {
        self.snapshots.last()
    }

    pub fn op_count(&self) -> usize {
        self.op_store.len()
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    fn should_snapshot(&self) -> bool {
        self.op_store.len().is_multiple_of(self.snapshot_interval)
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helios_crdt::{OpId, PeerId};

    fn make_insert(peer: PeerId, clock: u64, after: Option<OpId>, content: char) -> Op {
        Op::Insert {
            id: OpId::new(peer, clock),
            after,
            content,
        }
    }

    #[test]
    fn test_op_store_append_and_retrieve() {
        let mut store = OpStore::new();
        let peer = PeerId::nil();

        store.append(make_insert(peer, 1, None, 'a'));
        store.append(make_insert(peer, 2, Some(OpId::new(peer, 1)), 'b'));

        assert_eq!(store.len(), 2);

        let since_0 = store.get_since(0);
        assert_eq!(since_0.len(), 2);

        let since_1 = store.get_since(1);
        assert_eq!(since_1.len(), 1);
    }

    #[test]
    fn test_document_store_snapshot() {
        let mut store = DocumentStore::with_snapshot_interval(3);
        let mut doc = Document::new();
        let peer = PeerId::nil();

        for i in 1u8..=3 {
            let op = make_insert(
                peer,
                i as u64,
                if i == 1 {
                    None
                } else {
                    Some(OpId::new(peer, (i - 1) as u64))
                },
                (b'a' + i - 1) as char,
            );
            doc.apply(op.clone());
            store.apply_op(&doc, op);
        }

        assert_eq!(store.op_count(), 3);
        assert_eq!(store.snapshot_count(), 1);
        assert!(store.latest_snapshot().is_some());
    }

    #[test]
    fn test_document_store_load_from_snapshot() {
        let mut store = DocumentStore::with_snapshot_interval(3);
        let mut doc = Document::new();
        let peer = PeerId::nil();

        for i in 1u8..=3 {
            let op = make_insert(
                peer,
                i as u64,
                if i == 1 {
                    None
                } else {
                    Some(OpId::new(peer, (i - 1) as u64))
                },
                (b'a' + i - 1) as char,
            );
            doc.apply(op.clone());
            store.apply_op(&doc, op);
        }

        let loaded = store.load_document();
        assert_eq!(loaded.content(), doc.content());
    }

    #[test]
    fn test_document_store_replay_tail() {
        let mut store = DocumentStore::with_snapshot_interval(3);
        let mut doc = Document::new();
        let peer = PeerId::nil();

        // Create snapshot at op 3
        for i in 1u8..=3 {
            let op = make_insert(
                peer,
                i as u64,
                if i == 1 {
                    None
                } else {
                    Some(OpId::new(peer, (i - 1) as u64))
                },
                (b'a' + i - 1) as char,
            );
            doc.apply(op.clone());
            store.apply_op(&doc, op);
        }

        // Add more ops after snapshot
        for i in 4u8..=6 {
            let op = make_insert(
                peer,
                i as u64,
                Some(OpId::new(peer, (i - 1) as u64)),
                (b'a' + i - 1) as char,
            );
            doc.apply(op.clone());
            store.apply_op(&doc, op);
        }

        let loaded = store.load_document();
        assert_eq!(loaded.content(), "abcdef");
        assert_eq!(store.op_count(), 6);
        assert_eq!(store.snapshot_count(), 2);
    }

    #[test]
    fn test_document_store_empty() {
        let store = DocumentStore::new();
        let doc = store.load_document();
        assert_eq!(doc.content(), "");
    }
}
