use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

use crate::relay::transport::{TransportMeta, TransportRx, TransportTx};
use crate::structures::{TResult, TransportError};
use async_trait::async_trait;

fn strip_scheme(uri: &str) -> &str {
    uri.trim_start_matches("rist://")
}

fn parse_host_port(uri: &str) -> Option<SocketAddr> {
    // Ex: rist://127.0.0.1:11000?mode=caller
    let without_scheme = strip_scheme(uri);
    let host_port = without_scheme.split('?').next()?;
    if host_port.starts_with('@') {
        return None;
    }
    host_port.parse().ok()
}

fn is_listener_uri(uri: &str) -> bool {
    uri.contains("rist://@") || uri.split('?').nth(1).map(|q| q.contains("mode=listener")).unwrap_or(false)
}

fn describe_uri(prefix: &str, uri: &str) -> String {
    // Redact secrets before describing
    let red = crate::common::uri::redact_uri_secrets(uri);
    format!("{}={}", prefix, red)
}

pub struct RistReceiver {
    uri: String,
    sock: Option<UdpSocket>,
    bind_addr: SocketAddr,
}

pub struct RistSender {
    uri: String,
    sock: Option<UdpSocket>,
    target: SocketAddr,
}

impl RistReceiver {
    pub fn from_input_uri(uri: &str) -> TResult<Self> {
        let bind_addr: SocketAddr = if is_listener_uri(uri) || strip_scheme(uri).starts_with("@:") {
            let port = strip_scheme(uri)
                .trim_start_matches('@')
                .trim_start_matches(':')
                .split('?')
                .next()
                .ok_or_else(|| TransportError::InvalidUri(uri.into()))?
                .parse::<u16>()
                .map_err(|_| TransportError::InvalidUri(uri.into()))?;
            format!("0.0.0.0:{}", port).parse().unwrap()
        } else {
            // If a host:port is given on input, we still bind locally to that port to receive
            let host_port = strip_scheme(uri).split('?').next().unwrap();
            let mut parts = host_port.split(':');
            let _host = parts.next();
            let port: u16 = parts
                .next()
                .ok_or_else(|| TransportError::InvalidUri(uri.into()))?
                .parse()
                .map_err(|_| TransportError::InvalidUri(uri.into()))?;
            format!("0.0.0.0:{}", port).parse().unwrap()
        };
        Ok(Self { uri: uri.to_string(), sock: None, bind_addr })
    }
}

impl RistSender {
    pub fn from_output_uri(uri: &str) -> TResult<Self> {
        let target = parse_host_port(uri).ok_or_else(|| TransportError::InvalidUri(uri.into()))?;
        Ok(Self { uri: uri.to_string(), sock: None, target })
    }
}

#[async_trait]
impl TransportMeta for RistReceiver {
    fn open(&mut self) -> TResult<()> {
        let sock = std::net::UdpSocket::bind(self.bind_addr)?;
        sock.set_nonblocking(true)?;
        self.sock = Some(UdpSocket::from_std(sock)?);
        Ok(())
    }
    fn close(&mut self) {
        self.sock = None;
    }
    fn describe(&self) -> String {
        describe_uri("input", &self.uri)
    }
}

#[async_trait]
impl TransportRx for RistReceiver {
    async fn recv(&mut self, buf: &mut [u8]) -> TResult<usize> {
        let sock = self.sock.as_mut().ok_or(TransportError::Closed)?;
        match timeout(Duration::from_millis(20), sock.recv(buf)).await {
            Ok(Ok(n)) => Ok(n),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(TransportError::Timeout),
        }
    }
}

#[async_trait]
impl TransportMeta for RistSender {
    fn open(&mut self) -> TResult<()> {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
        sock.set_nonblocking(true)?;
        sock.connect(self.target)?;
        self.sock = Some(UdpSocket::from_std(sock)?);
        Ok(())
    }
    fn close(&mut self) {
        self.sock = None;
    }
    fn describe(&self) -> String {
        describe_uri("output", &self.uri)
    }
}

#[async_trait]
impl TransportTx for RistSender {
    async fn send(&mut self, buf: &[u8]) -> TResult<usize> {
        let sock = self.sock.as_mut().ok_or(TransportError::Closed)?;
        sock.send(buf).await.map_err(Into::into)
    }
}
