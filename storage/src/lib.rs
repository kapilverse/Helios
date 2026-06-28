use helios_crdt::Op;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("storage error: {0}")]
    Generic(String),
}

pub struct OpStore {
    ops: Vec<Op>,
}

impl OpStore {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn append(&mut self, op: Op) {
        self.ops.push(op);
    }

    pub fn get_since(&self, seq: u64) -> Vec<(u64, &Op)> {
        self.ops
            .iter()
            .enumerate()
            .skip(seq as usize)
            .map(|(i, op)| (i as u64, op))
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use helios_crdt::{OpId, PeerId};

    #[test]
    fn test_op_store_append_and_retrieve() {
        let mut store = OpStore::new();
        let peer = PeerId::nil();

        store.append(Op::Insert {
            id: OpId::new(peer, 1),
            after: None,
            content: 'a',
        });
        store.append(Op::Insert {
            id: OpId::new(peer, 2),
            after: Some(OpId::new(peer, 1)),
            content: 'b',
        });

        assert_eq!(store.len(), 2);

        let since_0 = store.get_since(0);
        assert_eq!(since_0.len(), 2);

        let since_1 = store.get_since(1);
        assert_eq!(since_1.len(), 1);
    }
}
