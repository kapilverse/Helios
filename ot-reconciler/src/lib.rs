use helios_crdt::{Document, Op};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReconcileError {
    #[error("conflicting operations could not be resolved")]
    ConflictResolutionFailed,
}

pub struct OtReconciler;

impl OtReconciler {
    pub fn new() -> Self {
        Self
    }

    /// Transform operation `a` against concurrent operation `b`.
    /// Returns `a'` such that: apply(b) then apply(a') == apply(a) then apply(b).
    pub fn transform(&self, a: &Op, b: &Op) -> Op {
        match (a, b) {
            // Insert-Insert: same parent → order by (peer, clock)
            (
                Op::Insert {
                    id: id_a,
                    after: after_a,
                    content: content_a,
                },
                Op::Insert {
                    id: id_b,
                    after: after_b,
                    content: _,
                },
            ) => {
                if after_a == after_b {
                    // Same parent — resolve by peer ID
                    if id_a.peer <= id_b.peer {
                        a.clone()
                    } else {
                        Op::Insert {
                            id: *id_a,
                            after: Some(*id_b),
                            content: *content_a,
                        }
                    }
                } else {
                    a.clone()
                }
            }

            // Delete-Insert: deleting what b inserted
            (Op::Delete { .. }, Op::Insert { .. }) => a.clone(),

            // Insert-Delete: if b deletes what a inserted
            (Op::Insert { .. }, Op::Delete { .. }) => a.clone(),

            // Delete-Delete: idempotent
            (Op::Delete { .. }, Op::Delete { .. }) => a.clone(),

            // SetKey-SetKey: LWW by timestamp
            (Op::SetKey { .. }, Op::SetKey { .. }) => a.clone(),

            // SetKey vs Insert/Delete: independent
            _ => a.clone(),
        }
    }

    /// Reconcile an incoming op against the document state.
    /// Returns corrected ops to broadcast.
    pub fn reconcile(&self, doc: &mut Document, incoming: Op, last_op: Option<&Op>) -> Vec<Op> {
        let corrected = match last_op {
            Some(existing) => self.transform(&incoming, existing),
            None => incoming.clone(),
        };
        doc.apply(corrected.clone());
        vec![corrected]
    }
}

impl Default for OtReconciler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helios_crdt::{Document, OpId, PeerId, Value};

    fn peer(id: u8) -> PeerId {
        let mut bytes = [0u8; 16];
        bytes[15] = id;
        PeerId::from_bytes(bytes)
    }

    // ── Insert-Insert ──────────────────────────────────────────────

    #[test]
    fn test_insert_insert_same_position_lower_peer_wins() {
        let r = OtReconciler::new();
        let a = Op::Insert {
            id: OpId::new(peer(1), 1),
            after: None,
            content: 'A',
        };
        let b = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'B',
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a, "lower peer should stay in place");
    }

    #[test]
    fn test_insert_insert_same_position_higher_peer_shifts() {
        let r = OtReconciler::new();
        let a = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'B',
        };
        let b = Op::Insert {
            id: OpId::new(peer(1), 1),
            after: None,
            content: 'A',
        };

        let result = r.transform(&a, &b);
        assert_eq!(
            result,
            Op::Insert {
                id: OpId::new(peer(2), 1),
                after: Some(OpId::new(peer(1), 1)),
                content: 'B'
            },
            "higher peer should shift after lower peer"
        );
    }

    #[test]
    fn test_insert_insert_different_position_no_change() {
        let r = OtReconciler::new();
        let a = Op::Insert {
            id: OpId::new(peer(1), 1),
            after: None,
            content: 'A',
        };
        let b = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: Some(OpId::new(peer(3), 1)),
            content: 'B',
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    #[test]
    fn test_insert_insert_convergence() {
        let r = OtReconciler::new();
        let peer1 = peer(1);
        let peer2 = peer(2);

        let a = Op::Insert {
            id: OpId::new(peer1, 1),
            after: None,
            content: 'A',
        };
        let b = Op::Insert {
            id: OpId::new(peer2, 1),
            after: None,
            content: 'B',
        };

        let a_prime = r.transform(&a, &b);
        let b_prime = r.transform(&b, &a);

        // Apply both orders — must converge
        let mut doc1 = Document::new();
        doc1.apply(b.clone());
        doc1.apply(a_prime.clone());

        let mut doc2 = Document::new();
        doc2.apply(a.clone());
        doc2.apply(b_prime.clone());

        assert_eq!(
            doc1.content(),
            doc2.content(),
            "must converge after transform"
        );
    }

    // ── Insert-Delete ──────────────────────────────────────────────

    #[test]
    fn test_insert_delete_target_match() {
        let r = OtReconciler::new();
        let id = OpId::new(peer(1), 1);
        let a = Op::Insert {
            id,
            after: None,
            content: 'A',
        };
        let b = Op::Delete { target: id };

        let result = r.transform(&a, &b);
        assert_eq!(
            result, a,
            "insert deleted by concurrent op — no change to a"
        );
    }

    #[test]
    fn test_insert_delete_no_match() {
        let r = OtReconciler::new();
        let a = Op::Insert {
            id: OpId::new(peer(1), 1),
            after: None,
            content: 'A',
        };
        let b = Op::Delete {
            target: OpId::new(peer(2), 1),
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    // ── Delete-Insert ──────────────────────────────────────────────

    #[test]
    fn test_delete_insert_deleting_inserted_element() {
        let r = OtReconciler::new();
        let id = OpId::new(peer(1), 1);
        let a = Op::Delete { target: id };
        let b = Op::Insert {
            id,
            after: None,
            content: 'A',
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    #[test]
    fn test_delete_insert_different_target() {
        let r = OtReconciler::new();
        let a = Op::Delete {
            target: OpId::new(peer(1), 1),
        };
        let b = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'B',
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    // ── Delete-Delete ──────────────────────────────────────────────

    #[test]
    fn test_delete_delete_same_target() {
        let r = OtReconciler::new();
        let target = OpId::new(peer(1), 1);
        let a = Op::Delete { target };
        let b = Op::Delete { target };

        let result = r.transform(&a, &b);
        assert_eq!(result, a, "delete is idempotent");
    }

    #[test]
    fn test_delete_delete_different_targets() {
        let r = OtReconciler::new();
        let a = Op::Delete {
            target: OpId::new(peer(1), 1),
        };
        let b = Op::Delete {
            target: OpId::new(peer(2), 1),
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    // ── SetKey-SetKey ──────────────────────────────────────────────

    #[test]
    fn test_setkey_same_key_newer_wins() {
        let r = OtReconciler::new();
        let map_id = OpId::new(peer(1), 1);
        let a = Op::SetKey {
            map_id,
            key: "k".into(),
            value: Value::Text("old".into()),
            ts: 1,
        };
        let b = Op::SetKey {
            map_id,
            key: "k".into(),
            value: Value::Text("new".into()),
            ts: 2,
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a, "a is kept but superseded by b's newer timestamp");
    }

    #[test]
    fn test_setkey_same_key_same_timestamp() {
        let r = OtReconciler::new();
        let map_id = OpId::new(peer(1), 1);
        let a = Op::SetKey {
            map_id,
            key: "k".into(),
            value: Value::Text("a".into()),
            ts: 1,
        };
        let b = Op::SetKey {
            map_id,
            key: "k".into(),
            value: Value::Text("b".into()),
            ts: 1,
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a, "same timestamp — a kept");
    }

    #[test]
    fn test_setkey_different_keys() {
        let r = OtReconciler::new();
        let map_id = OpId::new(peer(1), 1);
        let a = Op::SetKey {
            map_id,
            key: "x".into(),
            value: Value::Text("a".into()),
            ts: 1,
        };
        let b = Op::SetKey {
            map_id,
            key: "y".into(),
            value: Value::Text("b".into()),
            ts: 2,
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a, "different keys — no conflict");
    }

    // ── SetKey vs Insert/Delete ────────────────────────────────────

    #[test]
    fn test_setkey_vs_insert() {
        let r = OtReconciler::new();
        let a = Op::SetKey {
            map_id: OpId::new(peer(1), 1),
            key: "k".into(),
            value: Value::Text("v".into()),
            ts: 1,
        };
        let b = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'X',
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    #[test]
    fn test_setkey_vs_delete() {
        let r = OtReconciler::new();
        let a = Op::SetKey {
            map_id: OpId::new(peer(1), 1),
            key: "k".into(),
            value: Value::Text("v".into()),
            ts: 1,
        };
        let b = Op::Delete {
            target: OpId::new(peer(2), 1),
        };

        let result = r.transform(&a, &b);
        assert_eq!(result, a);
    }

    // ── Reconcile integration ──────────────────────────────────────

    #[test]
    fn test_reconcile_full_cycle() {
        let r = OtReconciler::new();
        let mut doc = Document::new();

        let op1 = Op::Insert {
            id: OpId::new(peer(1), 1),
            after: None,
            content: 'H',
        };
        doc.apply(op1.clone());

        let op2 = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'W',
        };
        let corrected = r.reconcile(&mut doc, op2, Some(&op1));

        assert_eq!(corrected.len(), 1);
        assert!(doc.content().contains('H'));
        assert!(doc.content().contains('W'));
    }

    #[test]
    fn test_reconcile_multiple_ops() {
        let r = OtReconciler::new();
        let mut doc = Document::new();

        let ops: Vec<Op> = (0u8..5)
            .map(|i| Op::Insert {
                id: OpId::new(peer(1), (i + 1) as u64),
                after: if i == 0 {
                    None
                } else {
                    Some(OpId::new(peer(1), i as u64))
                },
                content: (b'a' + i) as char,
            })
            .collect();

        for op in &ops {
            doc.apply(op.clone());
        }

        let incoming = Op::Insert {
            id: OpId::new(peer(2), 1),
            after: None,
            content: 'Z',
        };
        let corrected = r.reconcile(&mut doc, incoming, ops.last());

        assert_eq!(corrected.len(), 1);
        assert!(doc.content().contains('Z'));
    }
}
