use crate::structures::{TResult, TransportError};
use async_trait::async_trait;

// API commune minimale pour les transports de type « message » (SRT/RIST)
// Nota: l’implémentation V1 utilise UDP comme stub fonctionnel pour assurer un vrai débit local.

#[async_trait]
pub trait TransportRx: Send {
    // Lit des octets dans buf; Ok(n) avec n>0 si des données ont été reçues.
    // En cas de délai d’attente, renvoie TransportError::Timeout.
    async fn recv(&mut self, buf: &mut [u8]) -> TResult<usize>;
}

#[async_trait]
pub trait TransportTx: Send {
    // Envoie les n octets de buf; renvoie le nombre d’octets envoyés.
    async fn send(&mut self, buf: &[u8]) -> TResult<usize>;
}

pub trait TransportMeta {
    fn open(&mut self) -> TResult<()>;
    fn close(&mut self);
    fn describe(&self) -> String;
}
