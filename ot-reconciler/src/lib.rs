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

    pub fn transform(&self, a: &Op, b: &Op) -> Op {
        match (a, b) {
            (
                Op::Insert {
                    id: id_a,
                    after: after_a,
                    content: content_a,
                },
                Op::Insert {
                    id: id_b,
                    after: after_b,
                    content: _content_b,
                },
            ) => {
                if after_a == after_b {
                    // Same position — resolve by peer ID lexicographic order
                    if id_a.peer < id_b.peer {
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
            (Op::Delete { target: _ }, Op::Insert { .. }) => {
                // Delete wins over insert if target exists
                a.clone()
            }
            (Op::Insert { .. }, Op::Delete { .. }) => a.clone(),
            (Op::Delete { target: t1 }, Op::Delete { target: t2 }) => {
                if t1 == t2 {
                    a.clone()
                } else {
                    a.clone()
                }
            }
            _ => a.clone(),
        }
    }

    pub fn reconcile(&self, doc: &mut Document, incoming: Op, existing: &Op) -> Vec<Op> {
        let corrected = self.transform(&incoming, existing);
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
    use helios_crdt::{Document, OpId, PeerId};

    #[test]
    fn test_same_position_insert_ordering() {
        let reconciler = OtReconciler::new();
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

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

        let result = reconciler.transform(&a, &b);
        // peer1 < peer2, so a should be first
        assert_eq!(result, a);

        let result2 = reconciler.transform(&b, &a);
        // b should get after a's id
        assert_eq!(
            result2,
            Op::Insert {
                id: OpId::new(peer2, 1),
                after: Some(OpId::new(peer1, 1)),
                content: 'B',
            }
        );
    }

    #[test]
    fn test_reconcile_produces_corrected_ops() {
        let reconciler = OtReconciler::new();
        let mut doc = Document::new();
        let peer1 = PeerId::nil();
        let peer2 = PeerId::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let existing = Op::Insert {
            id: OpId::new(peer1, 1),
            after: None,
            content: 'A',
        };
        doc.apply(existing.clone());

        let incoming = Op::Insert {
            id: OpId::new(peer2, 1),
            after: None,
            content: 'B',
        };

        let corrected = reconciler.reconcile(&mut doc, incoming, &existing);
        assert_eq!(corrected.len(), 1);
        // Doc should contain both characters
        assert!(doc.content().contains('A'));
        assert!(doc.content().contains('B'));
    }
}
