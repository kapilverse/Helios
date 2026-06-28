use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    pub ops_sent: AtomicU64,
    pub ops_received: AtomicU64,
    pub corrections_emitted: AtomicU64,
    pub connections_active: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_op_sent(&self) {
        self.ops_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_op_received(&self) {
        self.ops_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_correction(&self) {
        self.corrections_emitted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_opened(&self) {
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            ops_sent: self.ops_sent.load(Ordering::Relaxed),
            ops_received: self.ops_received.load(Ordering::Relaxed),
            corrections_emitted: self.corrections_emitted.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub ops_sent: u64,
    pub ops_received: u64,
    pub corrections_emitted: u64,
    pub connections_active: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counters() {
        let m = Metrics::new();
        m.record_op_sent();
        m.record_op_sent();
        m.record_op_received();

        let snap = m.snapshot();
        assert_eq!(snap.ops_sent, 2);
        assert_eq!(snap.ops_received, 1);
    }
}
