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

    // Débits instantanés basés sur le dernier snapshot
    let (bps_out, mbps_recv) = metrics.instantaneous_rates();
    let protocol = metrics.protocol_string();

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

    Json(StatsResponse { protocol, data, status: "ok" })
}

// Endpoint Prometheus /metrics
#[get("/metrics")]
pub fn metrics_export(metrics: &State<Arc<Metrics>>) -> RawText<String> {
    RawText(metrics.gather_text())
}
