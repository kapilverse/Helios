use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PeerId = uuid::Uuid;
pub type Clock = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpId {
    pub peer: PeerId,
    pub clock: Clock,
}

impl OpId {
    pub fn new(peer: PeerId, clock: Clock) -> Self {
        Self { peer, clock }
    }
}

impl Ord for OpId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.clock
            .cmp(&other.clock)
            .then_with(|| self.peer.cmp(&other.peer))
    }
}

impl PartialOrd for OpId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Insert {
        id: OpId,
        after: Option<OpId>,
        content: char,
    },
    Delete {
        target: OpId,
    },
    SetKey {
        map_id: OpId,
        key: String,
        value: Value,
        ts: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Text(String),
    Bool(bool),
    Number(f64),
}

#[derive(Debug, Clone)]
struct Element {
    id: OpId,
    after: Option<OpId>,
    content: char,
    deleted: bool,
}

/// RGA (Replicated Growable Array) sequence CRDT.
/// Convergence: when two ops insert at the same position (same `after`),
/// ordering is deterministic by (peer_id, clock).
#[derive(Debug, Clone, Default)]
pub struct SequenceCrdt {
    elements: Vec<Element>,
    id_index: HashMap<OpId, usize>,
}

impl SequenceCrdt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_string(s: &str) -> Self {
        let mut crdt = Self::new();
        let peer = PeerId::nil();
        for (i, ch) in s.chars().enumerate() {
            let id = OpId::new(peer, (i + 1) as u64);
            let after = if i == 0 {
                None
            } else {
                Some(OpId::new(peer, i as u64))
            };
            crdt.elements.push(Element {
                id,
                after,
                content: ch,
                deleted: false,
            });
        }
        crdt.rebuild_index();
        crdt
    }

    pub fn apply(&mut self, op: Op) {
        match op {
            Op::Insert { id, after, content } => {
                let pos = self.find_insert_position(after, id);
                let element = Element {
                    id,
                    after,
                    content,
                    deleted: false,
                };
                self.elements.insert(pos, element);
                self.rebuild_index();
            }
            Op::Delete { target } => {
                if let Some(&idx) = self.id_index.get(&target) {
                    self.elements[idx].deleted = true;
                }
            }
            _ => {}
        }
    }

    pub fn as_string(&self) -> String {
        self.elements
            .iter()
            .filter(|e| !e.deleted)
            .map(|e| e.content)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.elements.iter().filter(|e| !e.deleted).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Find insert position: among siblings with same `after`,
    /// insert in (peer_id, clock) sorted order for determinism.
    fn find_insert_position(&self, after: Option<OpId>, new_id: OpId) -> usize {
        // Find all elements with the same 'after' value (siblings)
        let siblings: Vec<(usize, &Element)> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.deleted && e.after == after)
            .collect();

        if siblings.is_empty() {
            match after {
                None => 0,
                Some(after_id) => {
                    if let Some(&parent_idx) = self.id_index.get(&after_id) {
                        parent_idx + 1
                    } else {
                        self.elements.len()
                    }
                }
            }
        } else {
            // Find insertion point to maintain sorted order by (peer, clock)
            for &(idx, elem) in &siblings {
                if new_id < elem.id {
                    return idx;
                }
            }
            // New element has largest ID among siblings — insert after last
            siblings.last().unwrap().0 + 1
        }
    }

    fn rebuild_index(&mut self) {
        self.id_index.clear();
        for (i, elem) in self.elements.iter().enumerate() {
            self.id_index.insert(elem.id, i);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LwwMap {
    entries: HashMap<String, MapEntry>,
}

#[derive(Debug, Clone)]
struct MapEntry {
    value: Value,
    ts: u64,
}

impl LwwMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: String, value: Value, ts: u64) {
        let entry = self.entries.entry(key).or_insert_with(|| MapEntry {
            value: value.clone(),
            ts,
        });
        if ts >= entry.ts {
            entry.value = value;
            entry.ts = ts;
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|e| &e.value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OpLog {
    ops: Vec<Op>,
}

impl OpLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, op: Op) {
        self.ops.push(op);
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub sequence: SequenceCrdt,
    pub metadata: LwwMap,
    pub op_log: OpLog,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, op: Op) {
        match &op {
            Op::SetKey { key, value, ts, .. } => {
                self.metadata.set(key.clone(), value.clone(), *ts);
            }
            _ => {
                self.sequence.apply(op.clone());
            }
        }
        self.op_log.append(op);
    }

    pub fn content(&self) -> String {
        self.sequence.as_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::SliceRandom;
    use rand::Rng;

    #[test]
    fn test_op_id_ordering() {
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let a = OpId::new(peer1, 1);
        let b = OpId::new(peer2, 1);
        let c = OpId::new(peer1, 2);

        assert!(a < b);
        assert!(a < c);
    }

    #[test]
    fn test_rga_convergence_same_position() {
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let mut doc1 = Document::new();
        let mut doc2 = Document::new();

        let op_a = Op::Insert {
            id: OpId::new(peer1, 1),
            after: None,
            content: 'A',
        };
        let op_b = Op::Insert {
            id: OpId::new(peer2, 1),
            after: None,
            content: 'B',
        };

        // Apply in different orders — must converge
        doc1.apply(op_a.clone());
        doc1.apply(op_b.clone());

        doc2.apply(op_b.clone());
        doc2.apply(op_a.clone());

        assert_eq!(doc1.content(), doc2.content());
    }

    #[test]
    fn test_rga_sequential_inserts() {
        let peer = PeerId::nil();
        let mut doc = Document::new();

        doc.apply(Op::Insert {
            id: OpId::new(peer, 1),
            after: None,
            content: 'H',
        });
        doc.apply(Op::Insert {
            id: OpId::new(peer, 2),
            after: Some(OpId::new(peer, 1)),
            content: 'i',
        });

        assert_eq!(doc.content(), "Hi");
    }

    #[test]
    fn test_rga_delete() {
        let peer = PeerId::nil();
        let mut doc = Document::new();

        doc.apply(Op::Insert {
            id: OpId::new(peer, 1),
            after: None,
            content: 'H',
        });
        doc.apply(Op::Insert {
            id: OpId::new(peer, 2),
            after: Some(OpId::new(peer, 1)),
            content: 'i',
        });
        doc.apply(Op::Delete {
            target: OpId::new(peer, 1),
        });

        assert_eq!(doc.content(), "i");
    }

    #[test]
    fn test_rga_concurrent_interleaved() {
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let mut doc1 = Document::new();
        let mut doc2 = Document::new();

        // Both insert at start, then each appends
        let ops1 = vec![
            Op::Insert {
                id: OpId::new(peer1, 1),
                after: None,
                content: 'X',
            },
            Op::Insert {
                id: OpId::new(peer1, 2),
                after: Some(OpId::new(peer1, 1)),
                content: 'Y',
            },
        ];
        let ops2 = vec![
            Op::Insert {
                id: OpId::new(peer2, 1),
                after: None,
                content: 'A',
            },
            Op::Insert {
                id: OpId::new(peer2, 2),
                after: Some(OpId::new(peer2, 1)),
                content: 'B',
            },
        ];

        for op in &ops1 {
            doc1.apply(op.clone());
        }
        for op in &ops2 {
            doc1.apply(op.clone());
        }

        for op in &ops2 {
            doc2.apply(op.clone());
        }
        for op in &ops1 {
            doc2.apply(op.clone());
        }

        // Both must have all 4 characters
        let c1 = doc1.content();
        let c2 = doc2.content();
        assert!(c1.contains('X') && c1.contains('Y') && c1.contains('A') && c1.contains('B'));
        assert!(c2.contains('X') && c2.contains('Y') && c2.contains('A') && c2.contains('B'));
    }

    #[test]
    fn test_lww_map() {
        let mut map = LwwMap::new();
        map.set("key".to_string(), Value::Text("a".to_string()), 1);
        map.set("key".to_string(), Value::Text("b".to_string()), 2);
        assert_eq!(map.get("key"), Some(&Value::Text("b".to_string())));

        map.set("key".to_string(), Value::Text("c".to_string()), 1);
        assert_eq!(map.get("key"), Some(&Value::Text("b".to_string())));
    }

    #[test]
    fn test_op_log() {
        let mut log = OpLog::new();
        assert!(log.is_empty());

        log.append(Op::Insert {
            id: OpId::new(PeerId::nil(), 1),
            after: None,
            content: 'a',
        });

        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_convergence_fuzz() {
        use rand::prelude::SliceRandom;

        let mut rng = rand::thread_rng();
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        for _ in 0..100 {
            let mut doc1 = Document::new();
            let mut doc2 = Document::new();

            let mut all_ops: Vec<Op> = Vec::new();
            let mut clock1 = 0;
            let mut clock2 = 0;

            for _ in 0..20 {
                if rng.gen_bool(0.5) {
                    clock1 += 1;
                    all_ops.push(Op::Insert {
                        id: OpId::new(peer1, clock1),
                        after: if clock1 == 1 {
                            None
                        } else {
                            Some(OpId::new(peer1, clock1 - 1))
                        },
                        content: rng.gen::<char>(),
                    });
                } else {
                    clock2 += 1;
                    all_ops.push(Op::Insert {
                        id: OpId::new(peer2, clock2),
                        after: if clock2 == 1 {
                            None
                        } else {
                            Some(OpId::new(peer2, clock2 - 1))
                        },
                        content: rng.gen::<char>(),
                    });
                }
            }

            let mut shuffled = all_ops.clone();
            shuffled.shuffle(&mut rng);
            for op in &shuffled {
                doc1.apply(op.clone());
            }

            let mut shuffled2 = all_ops;
            shuffled2.shuffle(&mut rng);
            for op in &shuffled2 {
                doc2.apply(op.clone());
            }

            // Same multiset of characters
            let c1: Vec<char> = doc1.content().chars().collect();
            let c2: Vec<char> = doc2.content().chars().collect();
            assert_eq!(c1.len(), c2.len());

            let mut sorted1 = c1;
            let mut sorted2 = c2;
            sorted1.sort();
            sorted2.sort();
            assert_eq!(sorted1, sorted2);
        }
    }
}
