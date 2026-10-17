//! Astrazione del trasporto USB. `NusbTransport` la implementa sul dispositivo reale,
//! i test la implementano con un dispositivo simulato.

use std::time::Duration;

use crate::Result;

/// Trasferimenti di controllo vendor verso il dispositivo (recipient: device).
pub trait Transport {
    type Bulk: BulkSource;

    /// OUT, bmRequestType 0x40.
    fn control_out(&self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()>;

    /// IN, bmRequestType 0xc0. Restituisce i byte ricevuti, al massimo `len`.
    fn control_in(&self, request: u8, value: u16, index: u16, len: u16) -> Result<Vec<u8>>;

    /// Apre una coda di `queue_depth` trasferimenti bulk IN da `transfer_size` byte.
    /// I trasferimenti partono subito, prima del comando di avvio.
    fn open_bulk(&self, endpoint: u8, transfer_size: usize, queue_depth: usize) -> Result<Self::Bulk>;
}

/// Sorgente continua di dati bulk.
pub trait BulkSource {
    /// Attende il prossimo trasferimento completato e ne accoda i byte a `out`.
    /// Restituisce il numero di byte aggiunti. Rimette in coda il buffer.
    /// `Error::Timeout` se nulla arriva entro `timeout`.
    fn read_into(&mut self, out: &mut Vec<u8>, timeout: Duration) -> Result<usize>;

    /// Scarta i dati rimasti nel dispositivo da un'acquisizione precedente.
    /// Legge finché nulla arriva entro `quiet`. Restituisce i byte scartati.
    fn discard_stale(&mut self, quiet: Duration) -> Result<usize> {
        let _ = quiet;
        Ok(0)
    }
}
