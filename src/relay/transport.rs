// Traits de transport communs pour SRT et RIST (squelettes uniquement)
// Pour l'instant, ces traits définissent juste les signatures sans implémentation requise.

#[allow(dead_code)]
pub trait TransportRx {
    // Lecture d'un paquet/message (placeholder)
    fn poll_recv(&mut self) -> anyhow::Result<()>;
}

#[allow(dead_code)]
pub trait TransportTx {
    // Écriture d'un paquet/message (placeholder)
    fn poll_send(&mut self) -> anyhow::Result<()>;
}

#[allow(dead_code)]
pub trait Transport: TransportRx + TransportTx {
    // Ouvre la ressource (socket, session, etc.)
    fn open(&mut self) -> anyhow::Result<()>;
    // Ferme la ressource
    fn close(&mut self);
    // Description humaine de l'endpoint/connexion
    fn describe(&self) -> String;
}
