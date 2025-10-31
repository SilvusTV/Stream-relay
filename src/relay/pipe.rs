use crate::structures::{TResult, TransportError, Metrics};
use crate::relay::transport::{TransportMeta, TransportRx, TransportTx};
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use crate::common::logging::events;

pub async fn run_pipe<Rx, Tx>(mut rx: Rx, mut tx: Tx, protocol: &'static str, relay_id: &str) -> TResult<()>
where
    Rx: TransportRx + TransportMeta,
    Tx: TransportTx + TransportMeta,
{
    rx.open()?;
    tx.open()?;

    if let Some(m) = Metrics::global() {
        m.inc_active_relays();
        m.set_protocol(protocol);
    }

    info!(event = events::RELAY_START, subsystem = protocol, protocol = protocol, relay_id = %relay_id, input = %rx.describe(), output = %tx.describe(), msg = "Pipe start");

    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match rx.recv(&mut buf).await {
            Ok(n) if n > 0 => {
                if let Some(m) = Metrics::global() {
                    m.inc_pkt_in();
                    m.add_bytes_in(n as u64);
                }
                let sent = tx.send(&buf[..n]).await?;
                if let Some(m) = Metrics::global() {
                    m.inc_pkt_out();
                    m.add_bytes_out(sent as u64);
                }
            }
            Ok(_) => {
                // n == 0, ignore
            }
            Err(TransportError::Timeout) => {
                if let Some(m) = Metrics::global() { m.inc_timeout(); }
                sleep(Duration::from_millis(5)).await;
            }
            Err(e) => {
                error!(event = events::RELAY_ERROR, subsystem = protocol, protocol = protocol, relay_id = %relay_id, error = %e, msg = "Pipe error");
                if let Some(m) = Metrics::global() { m.dec_active_relays(); }
                break Err(e);
            }
        }
    }
}
