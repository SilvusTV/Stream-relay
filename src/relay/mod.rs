pub mod transport;
pub mod pipe;
pub mod srt;
pub mod rist;

use anyhow::Result;
use tokio::task::JoinHandle;
use tracing::{info, error};

use crate::relay::pipe::run_pipe;
use crate::relay::srt::{SrtReceiver, SrtSender};
use crate::relay::rist::{RistReceiver, RistSender};
use crate::common::logging::{events, short_uuid};
use crate::common::uri::redact_uri_secrets;

pub async fn run_srt_probe(input: String, output: String, latency_ms: u64) -> Result<()> {
    let relay_id = short_uuid();
    let red_in = redact_uri_secrets(&input);
    let red_out = redact_uri_secrets(&output);
    info!(event = events::RELAY_START, subsystem = "srt", protocol = "srt", relay_id = %relay_id, input = %red_in, output = %red_out, latency_ms = latency_ms, msg = "SRT probe start");
    let rx = SrtReceiver::from_input_uri(&input, latency_ms)?;
    let tx = SrtSender::from_output_uri(&output, latency_ms)?;
    // Boucle de pipe jusqu'à Ctrl+C
    if let Err(e) = run_pipe(rx, tx, "srt", &relay_id).await {
        error!(event = events::RELAY_ERROR, subsystem = "srt", protocol = "srt", relay_id = %relay_id, error = %e, msg = "SRT pipe error");
    }
    Ok(())
}

pub async fn run_rist_probe(input: String, output: String) -> Result<()> {
    let relay_id = short_uuid();
    let red_in = redact_uri_secrets(&input);
    let red_out = redact_uri_secrets(&output);
    info!(event = events::RELAY_START, subsystem = "rist", protocol = "rist", relay_id = %relay_id, input = %red_in, output = %red_out, msg = "RIST probe start");
    let rx = RistReceiver::from_input_uri(&input)?;
    let tx = RistSender::from_output_uri(&output)?;
    if let Err(e) = run_pipe(rx, tx, "rist", &relay_id).await {
        error!(event = events::RELAY_ERROR, subsystem = "rist", protocol = "rist", relay_id = %relay_id, error = %e, msg = "RIST pipe error");
    }
    Ok(())
}

// Auto-run background tasks that keep endpoints open and run the pipe in background
pub fn start_srt_auto(input: String, output: String, latency_ms: u64) -> JoinHandle<()> {
    tokio::spawn(async move {
        let relay_id = short_uuid();
        let red_in = redact_uri_secrets(&input);
        let red_out = redact_uri_secrets(&output);
        info!(event = events::RELAY_START, subsystem = "srt", protocol = "srt", relay_id = %relay_id, input = %red_in, output = %red_out, latency_ms = latency_ms, msg = "SRT auto start");
        let rx = match SrtReceiver::from_input_uri(&input, latency_ms) { Ok(v) => v, Err(e) => { error!(event = events::RELAY_ERROR, subsystem = "srt", protocol = "srt", relay_id = %relay_id, error = %e, msg = "SRT init rx failed"); return; } };
        let tx = match SrtSender::from_output_uri(&output, latency_ms) { Ok(v) => v, Err(e) => { error!(event = events::RELAY_ERROR, subsystem = "srt", protocol = "srt", relay_id = %relay_id, error = %e, msg = "SRT init tx failed"); return; } };
        if let Err(e) = run_pipe(rx, tx, "srt", &relay_id).await {
            error!(event = events::RELAY_ERROR, subsystem = "srt", protocol = "srt", relay_id = %relay_id, error = %e, msg = "SRT pipe error");
        }
    })
}

pub fn start_rist_auto(input: String, output: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let relay_id = short_uuid();
        let red_in = redact_uri_secrets(&input);
        let red_out = redact_uri_secrets(&output);
        info!(event = events::RELAY_START, subsystem = "rist", protocol = "rist", relay_id = %relay_id, input = %red_in, output = %red_out, msg = "RIST auto start");
        let rx = match RistReceiver::from_input_uri(&input) { Ok(v) => v, Err(e) => { error!(event = events::RELAY_ERROR, subsystem = "rist", protocol = "rist", relay_id = %relay_id, error = %e, msg = "RIST init rx failed"); return; } };
        let tx = match RistSender::from_output_uri(&output) { Ok(v) => v, Err(e) => { error!(event = events::RELAY_ERROR, subsystem = "rist", protocol = "rist", relay_id = %relay_id, error = %e, msg = "RIST init tx failed"); return; } };
        if let Err(e) = run_pipe(rx, tx, "rist", &relay_id).await {
            error!(event = events::RELAY_ERROR, subsystem = "rist", protocol = "rist", relay_id = %relay_id, error = %e, msg = "RIST pipe error");
        }
    })
}
