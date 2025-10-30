use std::time::Instant;
use rocket::{Data, Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use tracing::{info, debug};

use std::sync::Arc;
use crate::structures::Metrics;
use crate::common::logging::events;

pub mod routes;

// Fairing Rocket: intercepte chaque requête pour mesurer la durée et incrémenter les compteurs
pub struct HttpMetricsFairing;

#[rocket::async_trait]
impl Fairing for HttpMetricsFairing {
    fn info(&self) -> Info {
        Info { name: "HTTP metrics (counter + histogram + uptime)", kind: Kind::Request | Kind::Response }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        // Mémorise l'instant de début de traitement de la requête
        req.local_cache(|| Instant::now());
        // Génère ou récupère un request_id
        let req_id = req.headers().get_one("X-Request-ID").map(|s| s.to_string()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        req.local_cache(|| req_id);
        // Log debug http_request
        let method = req.method().as_str();
        let path = req.uri().path().to_string();
        let rid: &String = req.local_cache(|| String::new());
        debug!(event = events::HTTP_REQUEST, subsystem = "http", request_id = %rid, method = method, path = %path, msg = "HTTP request");
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // Récupère l'instant de début et calcule la durée
        let start = req.local_cache(|| Instant::now());
        let elapsed = start.elapsed();
        let method = req.method().as_str().to_string();
        let status_code = res.status().code;
        let status = status_code.to_string();

        if let Some(metrics) = req.rocket().state::<Arc<Metrics>>() {
            // Compte la requête par (méthode, statut)
            metrics.http_requests_total.with_label_values(&[&method, &status]).inc();
            // Observe la latence (en secondes) par méthode
            metrics.http_request_duration_seconds.with_label_values(&[&method]).observe(elapsed.as_secs_f64());
        }
        let rid: &String = req.local_cache(|| String::new());
        info!(event = events::HTTP_RESPONSE, subsystem = "http", request_id = %rid, method = %method, status = status_code, dur_ms = elapsed.as_millis() as u64, msg = "HTTP response");
    }
}
