use serde::Serialize;

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct StatsData {
    pub bitrate: i64,
    pub bytesRcvDrop: i64,
    pub bytesRcvLoss: i64,
    pub mbpsBandwidth: f64,
    pub mbpsRecvRate: f64,
    pub msRcvBuf: i64,
    pub pktRcvDrop: i64,
    pub pktRcvLoss: i64,
    pub rtt: f64,
    pub uptime: i64,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct StatsResponse {
    pub protocol: String,
    pub data: StatsData,
    pub status: &'static str,
}