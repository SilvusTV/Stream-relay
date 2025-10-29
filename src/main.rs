mod structures;
mod web;

use rocket::{routes, Rocket, Build};

// Constructeur de l'instance Rocket avec routes et fairings
fn build_rocket() -> Rocket<Build> {
    let metrics = structures::Metrics::new();

    rocket::build()
        .manage(metrics)
        .attach(web::HttpMetricsFairing)
        .mount(
            "/",
            routes![
                web::routes::health,
                web::routes::stats_endpoint,
                web::routes::metrics_export
            ],
        )
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // Exemple: initialisations avant le lancement (logs, ENV, tâches en arrière-plan, etc.)
    // ex: tracing_subscriber::fmt::init();

    build_rocket().launch().await?;
    Ok(())
}
