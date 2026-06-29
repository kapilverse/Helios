use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Default)]
pub struct Metrics {
    pub ops_sent: AtomicU64,
    pub ops_received: AtomicU64,
    pub corrections_emitted: AtomicU64,
    pub connections_active: AtomicU64,
    pub total_op_latency_ns: AtomicU64,
    pub op_latency_count: AtomicU64,
    pub convergence_time_ns: AtomicU64,
    pub convergence_count: AtomicU64,
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

    pub fn record_op_latency(&self, latency_ns: u64) {
        self.total_op_latency_ns
            .fetch_add(latency_ns, Ordering::Relaxed);
        self.op_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_convergence_time(&self, time_ns: u64) {
        self.convergence_time_ns
            .fetch_add(time_ns, Ordering::Relaxed);
        self.convergence_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn start_op_timer(&self) -> OpTimer<'_> {
        OpTimer {
            metrics: self,
            start: Instant::now(),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let op_count = self.op_latency_count.load(Ordering::Relaxed);
        let total_ns = self.total_op_latency_ns.load(Ordering::Relaxed);
        let avg_op_latency_us = total_ns
            .checked_div(op_count)
            .map(|v| v / 1_000)
            .unwrap_or(0);

        let conv_count = self.convergence_count.load(Ordering::Relaxed);
        let conv_total_ns = self.convergence_time_ns.load(Ordering::Relaxed);
        let avg_convergence_us = conv_total_ns
            .checked_div(conv_count)
            .map(|v| v / 1_000)
            .unwrap_or(0);

        MetricsSnapshot {
            ops_sent: self.ops_sent.load(Ordering::Relaxed),
            ops_received: self.ops_received.load(Ordering::Relaxed),
            corrections_emitted: self.corrections_emitted.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            avg_op_latency_us,
            avg_convergence_us,
            total_ops_measured: op_count,
        }
    }
}

pub struct OpTimer<'a> {
    metrics: &'a Metrics,
    start: Instant,
}

impl<'a> Drop for OpTimer<'a> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_nanos() as u64;
        self.metrics.record_op_latency(elapsed);
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub ops_sent: u64,
    pub ops_received: u64,
    pub corrections_emitted: u64,
    pub connections_active: u64,
    pub avg_op_latency_us: u64,
    pub avg_convergence_us: u64,
    pub total_ops_measured: u64,
}

impl std::fmt::Display for MetricsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Metrics: sent={}, received={}, corrections={}, connections={}, avg_latency={}µs, avg_convergence={}µs",
            self.ops_sent,
            self.ops_received,
            self.corrections_emitted,
            self.connections_active,
            self.avg_op_latency_us,
            self.avg_convergence_us,
        )
    }
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

    #[test]
    fn test_latency_tracking() {
        let m = Metrics::new();
        m.record_op_latency(1_000_000);
        m.record_op_latency(2_000_000);

        let snap = m.snapshot();
        assert_eq!(snap.avg_op_latency_us, 1500);
        assert_eq!(snap.total_ops_measured, 2);
    }

    #[test]
    fn test_convergence_tracking() {
        let m = Metrics::new();
        m.record_convergence_time(500_000);
        m.record_convergence_time(1_500_000);

        let snap = m.snapshot();
        assert_eq!(snap.avg_convergence_us, 1000);
    }

    #[test]
    fn test_connection_tracking() {
        let m = Metrics::new();
        m.connection_opened();
        m.connection_opened();
        m.connection_closed();

        let snap = m.snapshot();
        assert_eq!(snap.connections_active, 1);
    }
}
