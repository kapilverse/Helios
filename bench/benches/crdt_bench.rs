use criterion::{black_box, criterion_group, criterion_main, Criterion};
use helios_crdt::{Document, Op, OpId, PeerId};
use helios_ot_reconciler::OtReconciler;
use helios_storage::DocumentStore;

fn make_peer(id: u8) -> PeerId {
    let mut bytes = [0u8; 16];
    bytes[15] = id;
    PeerId::from_bytes(bytes)
}

fn bench_sequential_inserts(c: &mut Criterion) {
    let peer = make_peer(1);

    c.bench_function("sequential_inserts_1000", |b| {
        b.iter(|| {
            let mut doc = Document::new();
            for i in 0u64..1000 {
                let op = Op::Insert {
                    id: OpId::new(peer, i + 1),
                    after: if i == 0 { None } else { Some(OpId::new(peer, i)) },
                    content: 'a',
                };
                doc.apply(black_box(op));
            }
            doc
        });
    });
}

fn bench_concurrent_inserts(c: &mut Criterion) {
    let peer1 = make_peer(1);
    let peer2 = make_peer(2);

    c.bench_function("concurrent_inserts_1000", |b| {
        b.iter(|| {
            let mut doc = Document::new();
            for i in 0u64..500 {
                let op1 = Op::Insert {
                    id: OpId::new(peer1, i + 1),
                    after: None,
                    content: 'a',
                };
                let op2 = Op::Insert {
                    id: OpId::new(peer2, i + 1),
                    after: None,
                    content: 'b',
                };
                doc.apply(black_box(op1));
                doc.apply(black_box(op2));
            }
            doc
        });
    });
}

fn bench_convergence(c: &mut Criterion) {
    let peer1 = make_peer(1);
    let peer2 = make_peer(2);

    c.bench_function("convergence_1000_ops", |b| {
        b.iter(|| {
            let mut doc1 = Document::new();
            let mut doc2 = Document::new();
            for i in 0u64..500 {
                let op1 = Op::Insert {
                    id: OpId::new(peer1, i + 1),
                    after: None,
                    content: 'a',
                };
                let op2 = Op::Insert {
                    id: OpId::new(peer2, i + 1),
                    after: None,
                    content: 'b',
                };
                doc1.apply(op1.clone());
                doc1.apply(op2.clone());
                doc2.apply(op2);
                doc2.apply(op1);
            }
            assert_eq!(doc1.content(), doc2.content());
            doc1
        });
    });
}

fn bench_ot_transform(c: &mut Criterion) {
    let reconciler = OtReconciler::new();
    let peer1 = make_peer(1);
    let peer2 = make_peer(2);

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

    c.bench_function("ot_transform_10000", |b| {
        b.iter(|| {
            for _ in 0..10000 {
                black_box(reconciler.transform(&a, &b));
            }
        });
    });
}

fn bench_snapshot(c: &mut Criterion) {
    let peer = make_peer(1);

    c.bench_function("document_store_snapshot_1000", |b| {
        b.iter(|| {
            let mut store = DocumentStore::with_snapshot_interval(1000);
            let mut doc = Document::new();
            for i in 0u64..1000 {
                let op = Op::Insert {
                    id: OpId::new(peer, i + 1),
                    after: if i == 0 { None } else { Some(OpId::new(peer, i)) },
                    content: (b'a' + (i % 26) as u8) as char,
                };
                doc.apply(op.clone());
                store.apply_op(&doc, op);
            }
            store
        });
    });
}

fn bench_load_from_snapshot(c: &mut Criterion) {
    let peer = make_peer(1);
    let mut store = DocumentStore::with_snapshot_interval(1000);
    let mut doc = Document::new();
    for i in 0u64..5000 {
        let op = Op::Insert {
            id: OpId::new(peer, i + 1),
            after: if i == 0 { None } else { Some(OpId::new(peer, i)) },
            content: (b'a' + (i % 26) as u8) as char,
        };
        doc.apply(op.clone());
        store.apply_op(&doc, op);
    }

    c.bench_function("load_from_snapshot_5000", |b| {
        b.iter(|| {
            black_box(store.load_document());
        });
    });
}

criterion_group!(
    benches,
    bench_sequential_inserts,
    bench_concurrent_inserts,
    bench_convergence,
    bench_ot_transform,
    bench_snapshot,
    bench_load_from_snapshot,
);
criterion_main!(benches);
