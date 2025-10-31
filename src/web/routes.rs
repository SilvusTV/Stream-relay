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
// 
// Les données sont désormais DYNAMIQUES et calculées à partir des compteurs atomiques
// dans Metrics. Cette approche garantit la cohérence des statistiques en temps réel :
// - Utilise AtomicU64 pour des compteurs thread-safe sans verrous
// - Calcule les débits instantanés via des snapshots temporels
// - Estime le RTT basé sur les timeouts observés
// - Préparé pour l'intégration future des vraies librairies SRT/RIST
#[get("/stats")]
pub fn stats_endpoint(metrics: &State<Arc<Metrics>>) -> Json<StatsResponse> {
    let uptime_secs = metrics.start_time.elapsed().as_secs() as i64;
    metrics.uptime_seconds.set(uptime_secs);

    // Débits instantanés basés sur le dernier snapshot
    let (bps_out, mbps_recv) = metrics.instantaneous_rates();
    let protocol = metrics.protocol_string();

    // Update RTT estimate based on current state
    metrics.update_rtt_estimate();

    // Retrieve dynamic statistics
    let pkt_rcv_drop = metrics.get_pkt_rcv_drop();
    let pkt_rcv_loss = metrics.get_pkt_rcv_loss();
    let bytes_rcv_drop = metrics.get_bytes_rcv_drop();
    let bytes_rcv_loss = metrics.get_bytes_rcv_loss();
    let rtt = metrics.get_rtt_millis();
    let ms_rcv_buf = metrics.estimate_receive_buffer_ms();
    let mbps_bandwidth = metrics.estimate_bandwidth_mbps();

    let data = StatsData {
        bitrate: bps_out as i64,
        bytesRcvDrop: bytes_rcv_drop,
        bytesRcvLoss: bytes_rcv_loss,
        mbpsBandwidth: mbps_bandwidth,
        mbpsRecvRate: mbps_recv,
        msRcvBuf: ms_rcv_buf,
        pktRcvDrop: pkt_rcv_drop,
        pktRcvLoss: pkt_rcv_loss,
        rtt: rtt,
        uptime: uptime_secs,
    };

    Json(StatsResponse { protocol, data, status: "ok" })
}

// Endpoint Prometheus /metrics
#[get("/metrics")]
pub fn metrics_export(metrics: &State<Arc<Metrics>>) -> RawText<String> {
    RawText(metrics.gather_text())
}
