use helios_crdt::{Document, Op, OpId, PeerId};
use helios_ot_reconciler::OtReconciler;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostEntry {
    pub op: Op,
    pub confirmed: bool,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostSnapshot {
    pub content: String,
    pub confirmed_content: String,
    pub pending_count: usize,
    pub has_corrections: bool,
}

pub struct GhostEngine {
    pending: Vec<GhostEntry>,
    confirmed_doc: Document,
    optimistic_doc: Document,
    reconciler: OtReconciler,
    peer_id: PeerId,
    clock: u64,
}

thread_local! {
    static ENGINE: RefCell<Option<GhostEngine>> = const { RefCell::new(None) };
}

fn with_engine<R>(f: impl FnOnce(&mut GhostEngine) -> R) -> R {
    ENGINE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let engine = borrow
            .as_mut()
            .expect("GhostEngine not initialized — call init_engine() first");
        f(engine)
    })
}

#[wasm_bindgen]
pub fn init_engine(peer_id_str: &str) {
    let peer_id = PeerId::parse_str(peer_id_str).unwrap_or_else(|_| PeerId::nil());
    ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(GhostEngine {
            pending: Vec::new(),
            confirmed_doc: Document::new(),
            optimistic_doc: Document::new(),
            reconciler: OtReconciler::new(),
            peer_id,
            clock: 0,
        });
    });
    console_log!("Helios ghost engine initialized for peer {}", peer_id_str);
}

#[wasm_bindgen]
pub fn insert_char(after_op_id: &str, content: &str) -> JsValue {
    with_engine(|e| {
        e.clock += 1;

        let after = if after_op_id.is_empty() {
            None
        } else {
            parse_op_id(after_op_id)
        };

        let ch = content.chars().next().unwrap_or('\0');
        let op = Op::Insert {
            id: OpId::new(e.peer_id, e.clock),
            after,
            content: ch,
        };

        e.optimistic_doc.apply(op.clone());
        let entry = GhostEntry {
            op,
            confirmed: false,
            timestamp: js_sys::Date::now(),
        };
        let js = serde_wasm_bindgen::to_value(&entry).unwrap();
        e.pending.push(entry);
        js
    })
}

#[wasm_bindgen]
pub fn delete_char(target_op_id: &str) -> JsValue {
    with_engine(|e| {
        let target = parse_op_id(target_op_id).unwrap_or_else(|| OpId::new(e.peer_id, 0));
        let op = Op::Delete { target };

        e.optimistic_doc.apply(op.clone());
        let entry = GhostEntry {
            op,
            confirmed: false,
            timestamp: js_sys::Date::now(),
        };
        let js = serde_wasm_bindgen::to_value(&entry).unwrap();
        e.pending.push(entry);
        js
    })
}

#[wasm_bindgen]
pub fn get_content() -> String {
    with_engine(|e| e.optimistic_doc.content())
}

#[wasm_bindgen]
pub fn get_confirmed_content() -> String {
    with_engine(|e| e.confirmed_doc.content())
}

#[wasm_bindgen]
pub fn pending_count() -> usize {
    with_engine(|e| e.pending.iter().filter(|p| !p.confirmed).count())
}

#[wasm_bindgen]
pub fn apply_server_op(op_json: &str) -> JsValue {
    let op: Op = match serde_json::from_str(op_json) {
        Ok(o) => o,
        Err(err) => {
            console_log!("Failed to parse server op: {}", err);
            return JsValue::NULL;
        }
    };

    with_engine(|e| {
        let last_op = e.confirmed_doc.op_log.ops().last().cloned();
        let corrected = e
            .reconciler
            .reconcile(&mut e.confirmed_doc, op, last_op.as_ref());

        for corrected_op in &corrected {
            e.optimistic_doc.apply(corrected_op.clone());
        }

        e.pending.iter_mut().for_each(|p| {
            p.confirmed = true;
        });

        serde_wasm_bindgen::to_value(&corrected).unwrap()
    })
}

#[wasm_bindgen]
pub fn has_corrections() -> bool {
    with_engine(|e| e.optimistic_doc.content() != e.confirmed_doc.content())
}

#[wasm_bindgen]
pub fn snapshot() -> JsValue {
    with_engine(|e| {
        let s = GhostSnapshot {
            content: e.optimistic_doc.content(),
            confirmed_content: e.confirmed_doc.content(),
            pending_count: e.pending.iter().filter(|p| !p.confirmed).count(),
            has_corrections: e.optimistic_doc.content() != e.confirmed_doc.content(),
        };
        serde_wasm_bindgen::to_value(&s).unwrap()
    })
}

#[wasm_bindgen]
pub fn reset() {
    with_engine(|e| {
        e.pending.clear();
        e.confirmed_doc = Document::new();
        e.optimistic_doc = Document::new();
        e.clock = 0;
    });
}

fn parse_op_id(s: &str) -> Option<OpId> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let peer = PeerId::parse_str(parts[0]).ok()?;
    let clock: u64 = parts[1].parse().ok()?;
    Some(OpId::new(peer, clock))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_op_id() {
        let id = parse_op_id("00000000-0000-0000-0000-000000000001:5");
        assert!(id.is_some());
        let id = id.unwrap();
        assert_eq!(id.clock, 5);
    }

    #[test]
    fn test_parse_op_id_invalid() {
        assert!(parse_op_id("invalid").is_none());
        assert!(parse_op_id("00000000-0000-0000-0000-000000000001:notanumber").is_none());
    }
}
