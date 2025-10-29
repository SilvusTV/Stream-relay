use std::time::Instant;
use rocket::{Data, Request, Response};
use rocket::fairing::{Fairing, Info, Kind};

use crate::structures::Metrics;

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
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // Récupère l'instant de début et calcule la durée
        let start = req.local_cache(|| Instant::now());
        let elapsed = start.elapsed();
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
