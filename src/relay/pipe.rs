use crate::structures::{TResult, TransportError};
use crate::relay::transport::{TransportMeta, TransportRx, TransportTx};
use tokio::time::{sleep, Duration};

pub async fn run_pipe<Rx, Tx>(mut rx: Rx, mut tx: Tx) -> TResult<()>
where
    Rx: TransportRx + TransportMeta,
    Tx: TransportTx + TransportMeta,
{
    rx.open()?;
    tx.open()?;

    println!("[PIPE] start: {} -> {}", rx.describe(), tx.describe());

    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match rx.recv(&mut buf).await {
            Ok(n) if n > 0 => {
                let _ = tx.send(&buf[..n]).await?;
            }
            Ok(_) => {
                // n == 0, ignore
            }
            Err(TransportError::Timeout) => {
                sleep(Duration::from_millis(5)).await;
            }
            Err(e) => {
                eprintln!("[PIPE] error: {}", e);
                break Err(e);
            }
        }
    }
}
