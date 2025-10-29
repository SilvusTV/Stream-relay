use std::time::Instant;
use prometheus::{opts, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Registry, Encoder, TextEncoder};

// Regroupe le registry Prometheus et les métriques de l'application
pub struct Metrics {
    pub registry: Registry,
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub uptime_seconds: IntGauge,
    pub start_time: Instant,
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
        }
    }

    pub fn gather_text(&self) -> String {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).expect("encode metrics");
        String::from_utf8(buffer).unwrap_or_default()
    }
}

// Buckets d'histogramme adaptés à des latences HTTP (secondes)
fn duration_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25,
        0.5, 1.0, 2.5, 5.0,
    ]
}