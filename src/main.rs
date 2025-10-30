mod structures;
mod web;
mod relay;
mod common;

use clap::{Parser, Subcommand};
use rocket::{routes, Rocket, Build};
use rocket::fairing::AdHoc;
use tracing::{info, debug};
use crate::common::logging::{self, events};

// Constructeur de l'instance Rocket avec routes et fairings
fn build_rocket() -> Rocket<Build> {
    let metrics = std::sync::Arc::new(structures::Metrics::new());
    structures::Metrics::set_global(metrics.clone());

    rocket::build()
        .manage(metrics)
        .attach(web::HttpMetricsFairing)
        .attach(AdHoc::on_liftoff("auto-probes", |rocket| Box::pin(async move {
            // Lancement automatique des probes SRT/RIST après le démarrage du serveur HTTP
            // Les valeurs par défaut peuvent être surchargées via des variables d'environnement.
            // SRTRIST_AUTO_SRT=0 ou SRTRIST_AUTO_RIST=0 pour désactiver un protocole.
            // SRT: SRTRIST_SRT_INPUT, SRTRIST_SRT_OUTPUT, SRTRIST_SRT_LATENCY_MS
            // RIST: SRTRIST_RIST_INPUT, SRTRIST_RIST_OUTPUT
            #[cfg(feature = "srt")]
            {
                let auto = std::env::var("SRTRIST_AUTO_SRT").ok().map(|v| v != "0").unwrap_or(true);
                if auto {
                    let input = std::env::var("SRTRIST_SRT_INPUT").unwrap_or_else(|_| "srt://@:9000?mode=listener".to_string());
                    let output = std::env::var("SRTRIST_SRT_OUTPUT").unwrap_or_else(|_| "srt://127.0.0.1:10000?mode=caller".to_string());
                    let latency_ms: u64 = std::env::var("SRTRIST_SRT_LATENCY_MS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(80);
                    info!(event = events::RELAY_START, subsystem = "srt", protocol = "srt", msg = "Auto SRT probe enabled");
                    debug!(event = events::RELAY_START, subsystem = "srt", protocol = "srt", input = %input, output = %output, latency_ms = latency_ms, msg = "SRT defaults");
                    crate::relay::start_srt_auto(input, output, latency_ms);
                } else {
                    info!(event = events::RELAY_STOP, subsystem = "srt", protocol = "srt", msg = "Auto SRT probe disabled via SRTRIST_AUTO_SRT=0");
                }
            }

            #[cfg(feature = "rist")]
            {
                let auto = std::env::var("SRTRIST_AUTO_RIST").ok().map(|v| v != "0").unwrap_or(true);
                if auto {
                    let input = std::env::var("SRTRIST_RIST_INPUT").unwrap_or_else(|_| "rist://@:10000?mode=listener".to_string());
                    let output = std::env::var("SRTRIST_RIST_OUTPUT").unwrap_or_else(|_| "rist://127.0.0.1:11000?mode=caller".to_string());
                    info!(event = events::RELAY_START, subsystem = "rist", protocol = "rist", msg = "Auto RIST probe enabled");
                    debug!(event = events::RELAY_START, subsystem = "rist", protocol = "rist", input = %input, output = %output, msg = "RIST defaults");
                    crate::relay::start_rist_auto(input, output);
                } else {
                    info!(event = events::RELAY_STOP, subsystem = "rist", protocol = "rist", msg = "Auto RIST probe disabled via SRTRIST_AUTO_RIST=0");
                }
            }

            // Afficher l'adresse HTTP effective + URLs utiles
            let addr = rocket.config().address;
            let port = rocket.config().port;
            info!(event = events::APP_READY, subsystem = "http", msg = "HTTP server listening", address = %addr, port = port);
            debug!(event = events::APP_READY, subsystem = "http", msg = "Useful URLs", health = format!("http://{}:{}/health", addr, port), stats = format!("http://{}:{}/stats", addr, port), metrics = format!("http://{}:{}/metrics", addr, port));
        })))
        .attach(AdHoc::on_shutdown("log-shutdown", |_| Box::pin(async move {
            info!(event = events::APP_SHUTDOWN, msg = "Application shutting down");
        })))
        .mount(
            "/",
            routes![
                web::routes::health,
                web::routes::stats_endpoint,
                web::routes::metrics_export
            ],
        )
}

#[derive(Debug, Parser)]
#[command(name = "stream-relay", version, about = "Network stream relay with HTTP metrics")] 
struct Cli {
    /// Global: HTTP bind address (not yet wired)
    #[arg(long, global = true, default_value = "127.0.0.1:8000")]
    http_addr: String,
    /// Global: log level (not yet wired)
    #[arg(long, global = true, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Probe SRT input->output without piping payloads
    Srt2srt {
        /// Input URI (e.g., srt://@:9000?mode=listener)
        #[arg(long)]
        input: String,
        /// Output URI (e.g., srt://127.0.0.1:10000?mode=caller)
        #[arg(long)]
        output: String,
        /// Latency in milliseconds
        #[arg(long, default_value_t = 80)]
        latency_ms: u64,
    },
    /// Probe RIST input->output without piping payloads
    Rist2rist {
        /// Input URI (e.g., rist://@:9000?mode=listener)
        #[arg(long)]
        input: String,
        /// Output URI
        #[arg(long)]
        output: String,
    },
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // Init JSON logger (stdout)
    logging::init();

    // Minimal audit log at start
    info!(event = events::APP_START, msg = "Application starting", version = env!("CARGO_PKG_VERSION"), os = std::env::consts::OS);

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Srt2srt { input, output, latency_ms } => {
                if let Err(e) = relay::run_srt_probe(input, output, latency_ms).await {
                    tracing::error!(event = events::RELAY_ERROR, subsystem = "srt", protocol = "srt", error = %e, msg = "SRT probe failed");
                }
                return Ok(());
            }
            Commands::Rist2rist { input, output } => {
                if let Err(e) = relay::run_rist_probe(input, output).await {
                    tracing::error!(event = events::RELAY_ERROR, subsystem = "rist", protocol = "rist", error = %e, msg = "RIST probe failed");
                }
                return Ok(());
            }
        }
    }

    build_rocket().launch().await?;
    Ok(())
}
