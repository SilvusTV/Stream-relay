// Serveur HTTP Rocket exposant /health et /stats (format Prometheus)
// Instrumentation: compteur total par méthode/statut, histogramme de latence par méthode, jauge d'uptime
use std::time::{Duration, Instant};
use rocket::{fairing::{Fairing, Info, Kind}, Request, Data, Response, State};
use rocket::{get, launch, routes};
use rocket::serde::json::Json;
use serde::Serialize;
use prometheus::{Registry, IntCounterVec, IntGauge, HistogramVec, HistogramOpts, opts};

// Regroupe le registry Prometheus et les métriques de l'application
struct Metrics {
    registry: Registry,
    http_requests_total: IntCounterVec,
    http_request_duration_seconds: HistogramVec,
    uptime_seconds: IntGauge,
    start_time: Instant,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

// Endpoint de santé: renvoie un JSON minimal { "status": "ok" }
#[get("/health")]
fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct StatsData {
    bitrate: i64,
    bytesRcvDrop: i64,
    bytesRcvLoss: i64,
    mbpsBandwidth: f64,
    mbpsRecvRate: f64,
    msRcvBuf: i64,
    pktRcvDrop: i64,
    pktRcvLoss: i64,
    rtt: f64,
    uptime: i64,
}

#[derive(Serialize)]
struct StatsResponse {
    data: StatsData,
    status: &'static str,
}


// Endpoint stats: renvoie un JSON complet (format inspiré de TemplateStatsResponse.json)
#[get("/stats")]
fn stats_endpoint(metrics: &State<Metrics>) -> Json<StatsResponse> {
    let uptime_secs = metrics.start_time.elapsed().as_secs() as i64;
    metrics.uptime_seconds.set(uptime_secs);

    // Valeurs par défaut (à remplacer plus tard par des valeurs réelles)
    
    //@TODO: Emplacement des valeurs par défaut Metrics/Stats
    let data = StatsData {
        bitrate: 0,
        bytesRcvDrop: 0,
        bytesRcvLoss: 0,
        mbpsBandwidth: 0.0,
        mbpsRecvRate: 0.0,
        msRcvBuf: 0,
        pktRcvDrop: 0,
        pktRcvLoss: 0,
        rtt: 0.0,
        uptime: uptime_secs,
    };

    Json(StatsResponse { data, status: "ok" })
}

// Fairing Rocket: intercepte chaque requête pour mesurer la durée et incrémenter les compteurs
struct HttpMetricsFairing;

#[rocket::async_trait]
impl Fairing for HttpMetricsFairing {
    fn info(&self) -> Info {
        Info { name: "HTTP metrics (counter + histogram + uptime)", kind: Kind::Request | Kind::Response }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        // Mémorise l'instant de début de traitement de la requête
        req.local_cache(|| Instant::now());
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // Récupère l'instant de début et calcule la durée
        let start = req.local_cache(|| Instant::now());
        let elapsed: Duration = start.elapsed();
        let method = req.method().as_str().to_string();
        let status = res.status().code.to_string();

        if let Some(metrics) = req.rocket().state::<Metrics>() {
            // Compte la requête par (méthode, statut)
            metrics.http_requests_total.with_label_values(&[&method, &status]).inc();
            // Observe la latence (en secondes) par méthode
            metrics.http_request_duration_seconds.with_label_values(&[&method]).observe(elapsed.as_secs_f64());
        }
    }
}

// Buckets d'histogramme adaptés à des latences HTTP (secondes)
fn duration_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25,
        0.5, 1.0, 2.5, 5.0,
    ]
}

#[launch]
fn rocket() -> _ {
    // Création du registry et des métriques
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

    // Enregistrement des métriques dans le registry
    registry.register(Box::new(http_requests_total.clone())).expect("register counter vec");
    registry.register(Box::new(http_request_duration_seconds.clone())).expect("register histogram vec");
    registry.register(Box::new(uptime_seconds.clone())).expect("register gauge");

    let metrics = Metrics {
        registry,
        http_requests_total,
        http_request_duration_seconds,
        uptime_seconds,
        start_time: Instant::now(),
    };

    // Construction et lancement de l'application Rocket
    rocket::build()
        .manage(metrics)
        .attach(HttpMetricsFairing)
        .mount("/", routes![health, stats_endpoint])
}
