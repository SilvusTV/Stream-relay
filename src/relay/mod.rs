pub mod transport;
pub mod pipe;
pub mod srt;
pub mod rist;

use anyhow::Result;
use tokio::task::JoinHandle;

use crate::relay::pipe::run_pipe;
use crate::relay::srt::{SrtReceiver, SrtSender};
use crate::relay::rist::{RistReceiver, RistSender};

pub async fn run_srt_probe(input: String, output: String, latency_ms: u64) -> Result<()> {
    println!("[INIT] SRT input={} output={} latency_ms={}", input, output, latency_ms);
    let rx = SrtReceiver::from_input_uri(&input, latency_ms)?;
    let tx = SrtSender::from_output_uri(&output, latency_ms)?;
    // Boucle de pipe jusqu'à Ctrl+C
    if let Err(e) = run_pipe(rx, tx).await {
        eprintln!("[SRT] pipe error: {e}");
    }
    Ok(())
}

pub async fn run_rist_probe(input: String, output: String) -> Result<()> {
    println!("[INIT] RIST input={} output={}", input, output);
    let rx = RistReceiver::from_input_uri(&input)?;
    let tx = RistSender::from_output_uri(&output)?;
    if let Err(e) = run_pipe(rx, tx).await {
        eprintln!("[RIST] pipe error: {e}");
    }
    Ok(())
}

// Auto-run background tasks that keep endpoints open and run the pipe in background
pub fn start_srt_auto(input: String, output: String, latency_ms: u64) -> JoinHandle<()> {
    tokio::spawn(async move {
        println!("[INIT] SRT auto: input={} output={} latency_ms={}", input, output, latency_ms);
        let rx = match SrtReceiver::from_input_uri(&input, latency_ms) { Ok(v) => v, Err(e) => { eprintln!("[SRT] init rx failed: {e}"); return; } };
        let tx = match SrtSender::from_output_uri(&output, latency_ms) { Ok(v) => v, Err(e) => { eprintln!("[SRT] init tx failed: {e}"); return; } };
        if let Err(e) = run_pipe(rx, tx).await {
            eprintln!("[SRT] pipe error: {e}");
        }
    })
}

pub fn start_rist_auto(input: String, output: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        println!("[INIT] RIST auto: input={} output={}", input, output);
        let rx = match RistReceiver::from_input_uri(&input) { Ok(v) => v, Err(e) => { eprintln!("[RIST] init rx failed: {e}"); return; } };
        let tx = match RistSender::from_output_uri(&output) { Ok(v) => v, Err(e) => { eprintln!("[RIST] init tx failed: {e}"); return; } };
        if let Err(e) = run_pipe(rx, tx).await {
            eprintln!("[RIST] pipe error: {e}");
        }
    })
}
