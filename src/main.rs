mod structures;
mod web;
mod relay;

use clap::{Parser, Subcommand};
use rocket::{routes, Rocket, Build};
use rocket::fairing::AdHoc;

// Constructeur de l'instance Rocket avec routes et fairings
fn build_rocket() -> Rocket<Build> {
    let metrics = structures::Metrics::new();

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
                    println!("[INIT] Auto SRT probe enabled");
                    println!("[INIT] SRT defaults: input={} output={} latency_ms={}", input, output, latency_ms);
                    crate::relay::start_srt_auto(input, output, latency_ms);
                } else {
                    println!("[INIT] Auto SRT probe disabled via SRTRIST_AUTO_SRT=0");
                }
            }

            #[cfg(feature = "rist")]
            {
                let auto = std::env::var("SRTRIST_AUTO_RIST").ok().map(|v| v != "0").unwrap_or(true);
                if auto {
                    let input = std::env::var("SRTRIST_RIST_INPUT").unwrap_or_else(|_| "rist://@:10000?mode=listener".to_string());
                    let output = std::env::var("SRTRIST_RIST_OUTPUT").unwrap_or_else(|_| "rist://127.0.0.1:11000?mode=caller".to_string());
                    println!("[INIT] Auto RIST probe enabled");
                    println!("[INIT] RIST defaults: input={} output={}", input, output);
                    crate::relay::start_rist_auto(input, output);
                } else {
                    println!("[INIT] Auto RIST probe disabled via SRTRIST_AUTO_RIST=0");
                }
            }

            // Afficher l'adresse HTTP effective + URLs utiles
            let addr = rocket.config().address;
            let port = rocket.config().port;
            println!("[INIT] HTTP server listening on {}:{}", addr, port);
            println!("[INFO] URLs: http://{}:{}/health  http://{}:{}/stats  http://{}:{}/metrics", addr, port, addr, port, addr, port);
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
    // Exemple: initialisations avant le lancement (logs, ENV, tâches en arrière-plan, etc.)
    // ex: tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Srt2srt { input, output, latency_ms } => {
                if let Err(e) = relay::run_srt_probe(input, output, latency_ms).await {
                    eprintln!("SRT probe failed: {e}");
                }
                return Ok(());
            }
            Commands::Rist2rist { input, output } => {
                if let Err(e) = relay::run_rist_probe(input, output).await {
                    eprintln!("RIST probe failed: {e}");
                }
                return Ok(());
            }
        }
    }

    build_rocket().launch().await?;
    Ok(())
}
