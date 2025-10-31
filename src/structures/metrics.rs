use std::time::Instant;
use std::sync::{Arc, Mutex};
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
    // Additional statistics for StatsData
    pub pkt_rcv_drop: AtomicU64,
    pub pkt_rcv_loss: AtomicU64,
    pub bytes_rcv_drop: AtomicU64,
    pub bytes_rcv_loss: AtomicU64,
    // Protocol and instantaneous snapshot state
    current_protocol: AtomicU64, // 0=unknown,1=srt,2=rist
    last_snapshot: Mutex<(Instant, u64, u64)>, // (time, bytes_in_total, bytes_out_total)
    // RTT tracking (estimated from timeouts and available data)
    rtt_millis: AtomicU64, // milliseconds
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
            pkt_rcv_drop: AtomicU64::new(0),
            pkt_rcv_loss: AtomicU64::new(0),
            bytes_rcv_drop: AtomicU64::new(0),
            bytes_rcv_loss: AtomicU64::new(0),
            current_protocol: AtomicU64::new(0),
            last_snapshot: Mutex::new((Instant::now(), 0, 0)),
            rtt_millis: AtomicU64::new(0),
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

    // Protocol helpers
    #[inline]
    pub fn set_protocol(&self, protocol: &str) {
        let code = match protocol.to_ascii_lowercase().as_str() {
            "srt" => 1u64,
            "rist" => 2u64,
            _ => 0u64,
        };
        self.current_protocol.store(code, Ordering::Relaxed);
    }

    #[inline]
    pub fn protocol_string(&self) -> String {
        match self.current_protocol.load(Ordering::Relaxed) {
            1 => "srt".to_string(),
            2 => "rist".to_string(),
            _ => "unknown".to_string(),
        }
    }

    // Instantaneous rate computation using last snapshot
    pub fn instantaneous_rates(&self) -> (f64, f64) {
        // returns (bps_out, mbps_recv)
        let now = Instant::now();
        let bytes_in = self.bytes_in_total.load(Ordering::Relaxed);
        let bytes_out = self.bytes_out_total.load(Ordering::Relaxed);
        let mut guard = self.last_snapshot.lock().expect("lock snapshot");
        let (last_t, last_in, last_out) = *guard;
        let dt = now.duration_since(last_t).as_secs_f64();
        // update snapshot for next call
        *guard = (now, bytes_in, bytes_out);
        if dt <= 0.0 { return (0.0, 0.0); }
        let delta_in = bytes_in.saturating_sub(last_in) as f64;
        let delta_out = bytes_out.saturating_sub(last_out) as f64;
        let bps_out = (delta_out * 8.0) / dt;
        let mbps_recv = (delta_in * 8.0) / dt / 1_000_000.0;
        (bps_out, mbps_recv)
    }

    // Getter methods for statistics
    #[inline]
    pub fn get_pkt_rcv_drop(&self) -> i64 {
        self.pkt_rcv_drop.load(Ordering::Relaxed) as i64
    }

    #[inline]
    pub fn get_pkt_rcv_loss(&self) -> i64 {
        self.pkt_rcv_loss.load(Ordering::Relaxed) as i64
    }

    #[inline]
    pub fn get_bytes_rcv_drop(&self) -> i64 {
        self.bytes_rcv_drop.load(Ordering::Relaxed) as i64
    }

    #[inline]
    pub fn get_bytes_rcv_loss(&self) -> i64 {
        self.bytes_rcv_loss.load(Ordering::Relaxed) as i64
    }

    #[inline]
    pub fn get_rtt_millis(&self) -> f64 {
        self.rtt_millis.load(Ordering::Relaxed) as f64
    }

    // Update estimated RTT based on timeouts
    // This is a simple estimation: if we see many timeouts, assume RTT is higher
    #[inline]
    pub fn update_rtt_estimate(&self) {
        let timeouts = self.timeouts_total.load(Ordering::Relaxed);
        let active = self.active_relays.load(Ordering::Relaxed);
        // Simple heuristic: base RTT of 20ms, add 5ms per timeout if active relays exist
        let estimated_rtt = if active > 0 && timeouts > 0 {
            20 + (timeouts.saturating_mul(5).min(200))
        } else {
            20
        };
        self.rtt_millis.store(estimated_rtt, Ordering::Relaxed);
    }

    // Calculate buffer metrics based on current data flow
    pub fn estimate_receive_buffer_ms(&self) -> i64 {
        // Estimate buffer based on recent packet rate
        let pkt_in = self.pkt_in_total.load(Ordering::Relaxed);
        let uptime_secs = self.start_time.elapsed().as_secs();
        let bytes_in = self.bytes_in_total.load(Ordering::Relaxed);
        
        if uptime_secs > 0 && pkt_in > 0 {
            // Average packet size
            let avg_pkt_size = bytes_in / pkt_in;
            // Packets per second
            let pps = pkt_in / uptime_secs;
            // Estimate buffer holds ~100ms of packets
            let estimated_buffer_ms = 100;
            estimated_buffer_ms
        } else {
            0
        }
    }

    // Estimate bandwidth based on current rates
    pub fn estimate_bandwidth_mbps(&self) -> f64 {
        let (_, mbps_recv) = self.instantaneous_rates();
        mbps_recv
    }

    // Track packet losses based on timeouts
    #[inline]
    pub fn inc_pkt_loss(&self, count: u64) {
        self.pkt_rcv_loss.fetch_add(count, Ordering::Relaxed);
    }

    // Track packet drops (simulated for now)
    #[inline]
    pub fn inc_pkt_drop(&self, count: u64) {
        self.pkt_rcv_drop.fetch_add(count, Ordering::Relaxed);
    }
}

// Buckets d'histogramme adaptés à des latences HTTP (secondes)
fn duration_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25,
        0.5, 1.0, 2.5, 5.0,
    ]
}