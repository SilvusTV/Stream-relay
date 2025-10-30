use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use once_cell::sync::OnceCell;
use prometheus::{opts, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Registry, Encoder, TextEncoder};

// Global handle to metrics for non-HTTP contexts (e.g., relay pipe)
pub static GLOBAL_METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();

// Regroupe le registry Prometheus et les métriques de l'application
pub struct Metrics {
    pub registry: Registry,
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub uptime_seconds: IntGauge,
    pub start_time: Instant,
    // Runtime I/O stats aggregated across all pipes
    pub bytes_in_total: AtomicU64,
    pub bytes_out_total: AtomicU64,
    pub pkt_in_total: AtomicU64,
    pub pkt_out_total: AtomicU64,
    pub timeouts_total: AtomicU64,
    pub active_relays: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let http_requests_total = IntCounterVec::new(
            opts!("http_requests_total", "Total HTTP requests by method and status"),
            &["method", "status"],
        ).expect("create counter vec");

        let histogram_opts = HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request latencies in seconds",
        ).buckets(duration_buckets());

        let http_request_duration_seconds = HistogramVec::new(
            histogram_opts,
            &["method"],
        ).expect("create histogram vec");

        let uptime_seconds = IntGauge::new("uptime_seconds", "Process uptime in seconds")
            .expect("create gauge");

        registry.register(Box::new(http_requests_total.clone())).expect("register counter vec");
        registry.register(Box::new(http_request_duration_seconds.clone())).expect("register histogram vec");
        registry.register(Box::new(uptime_seconds.clone())).expect("register gauge");

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            uptime_seconds,
            start_time: Instant::now(),
            bytes_in_total: AtomicU64::new(0),
            bytes_out_total: AtomicU64::new(0),
            pkt_in_total: AtomicU64::new(0),
            pkt_out_total: AtomicU64::new(0),
            timeouts_total: AtomicU64::new(0),
            active_relays: AtomicU64::new(0),
        }
    }

    pub fn set_global(arc: Arc<Metrics>) {
        let _ = GLOBAL_METRICS.set(arc);
    }

    pub fn global() -> Option<&'static Arc<Metrics>> {
        GLOBAL_METRICS.get()
    }

    pub fn gather_text(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).expect("encode metrics");
        String::from_utf8(buffer).unwrap_or_default()
    }

    // Convenience helpers
    #[inline]
    pub fn inc_active_relays(&self) { self.active_relays.fetch_add(1, Ordering::SeqCst); }
    #[inline]
    pub fn dec_active_relays(&self) { self.active_relays.fetch_sub(1, Ordering::SeqCst); }
    #[inline]
    pub fn add_bytes_in(&self, n: u64) { self.bytes_in_total.fetch_add(n, Ordering::Relaxed); }
    #[inline]
    pub fn add_bytes_out(&self, n: u64) { self.bytes_out_total.fetch_add(n, Ordering::Relaxed); }
    #[inline]
    pub fn inc_pkt_in(&self) { self.pkt_in_total.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn inc_pkt_out(&self) { self.pkt_out_total.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn inc_timeout(&self) { self.timeouts_total.fetch_add(1, Ordering::Relaxed); }
}

// Buckets d'histogramme adaptés à des latences HTTP (secondes)
fn duration_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25,
        0.5, 1.0, 2.5, 5.0,
    ]
}