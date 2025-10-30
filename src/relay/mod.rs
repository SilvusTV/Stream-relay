pub mod transport;
pub mod srt;
pub mod rist;

use anyhow::Result;
use tokio::time::{sleep, Duration};
use tokio::task::JoinHandle;

use srt::SrtEndpoint;
use rist::RistEndpoint;

pub async fn run_srt_probe(input: String, output: String, latency_ms: u64) -> Result<()> {
    let mut ep = SrtEndpoint::new(input, output, latency_ms);
    ep.open()?;
    println!("{}", ep.describe());
    sleep(Duration::from_secs(3)).await;
    ep.close();
    println!("{}", ep.describe());
    Ok(())
}

pub async fn run_rist_probe(input: String, output: String) -> Result<()> {
    let mut ep = RistEndpoint::new(input, output);
    ep.open()?;
    println!("{}", ep.describe());
    sleep(Duration::from_secs(3)).await;
    ep.close();
    println!("{}", ep.describe());
    Ok(())
}

// Auto-run background tasks that keep endpoints open and periodically log their state
pub fn start_srt_auto(input: String, output: String, latency_ms: u64) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let mut ep = SrtEndpoint::new(input.clone(), output.clone(), latency_ms);
            if let Err(e) = ep.open() {
                eprintln!("[SRT] failed to open endpoint: {e}");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            println!("{}", ep.describe());
            // Keep reporting status periodically until future update adds real sockets
            for _ in 0..12u8 { // ~12 minutes at 60s interval as a placeholder
                sleep(Duration::from_secs(60)).await;
                println!("{}", ep.describe());
            }
            // Close and loop (placeholder behaviour); in real code we would await shutdown
            ep.close();
            println!("{}", ep.describe());
            sleep(Duration::from_secs(5)).await;
        }
    })
}

pub fn start_rist_auto(input: String, output: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let mut ep = RistEndpoint::new(input.clone(), output.clone());
            if let Err(e) = ep.open() {
                eprintln!("[RIST] failed to open endpoint: {e}");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            println!("{}", ep.describe());
            for _ in 0..12u8 {
                sleep(Duration::from_secs(60)).await;
                println!("{}", ep.describe());
            }
            ep.close();
            println!("{}", ep.describe());
            sleep(Duration::from_secs(5)).await;
        }
    })
}
