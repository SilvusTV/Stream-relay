use rocket::serde::json::Json;
use rocket::get;
use rocket::State;
use rocket::response::content::RawText;
use std::sync::Arc;

use crate::structures::{HealthResponse, Metrics, StatsData, StatsResponse};

// Endpoint de santé: renvoie un JSON minimal { "status": "ok" }
#[get("/health")]
pub fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", code: 200 })
}

// Endpoint stats: renvoie un JSON complet (format inspiré de TemplateStatsResponse.json)
#[get("/stats")]
pub fn stats_endpoint(metrics: &State<Arc<Metrics>>) -> Json<StatsResponse> {
    let uptime_secs = metrics.start_time.elapsed().as_secs() as i64;
    metrics.uptime_seconds.set(uptime_secs);

    // Agrégation simple depuis les compteurs globaux
    let bytes_in = metrics.bytes_in_total.load(std::sync::atomic::Ordering::Relaxed) as f64;
    let bytes_out = metrics.bytes_out_total.load(std::sync::atomic::Ordering::Relaxed) as f64;

    let seconds = uptime_secs.max(1) as f64;
    let bps_out = (bytes_out * 8.0) / seconds; // bitrate moyen sortant en bps
    let mbps_recv = (bytes_in * 8.0) / seconds / 1_000_000.0; // Mbps moyen entrant

    let data = StatsData {
        bitrate: bps_out as i64,
        bytesRcvDrop: 0,
        bytesRcvLoss: 0,
        mbpsBandwidth: 0.0,
        mbpsRecvRate: mbps_recv,
        msRcvBuf: 0,
        pktRcvDrop: 0,
        pktRcvLoss: 0,
        rtt: 0.0,
        uptime: uptime_secs,
    };

    Json(StatsResponse { data, status: "ok" })
}

// Endpoint Prometheus /metrics
#[get("/metrics")]
pub fn metrics_export(metrics: &State<Arc<Metrics>>) -> RawText<String> {
    RawText(metrics.gather_text())
}
