//! Sorgente di campioni in volt: il dispositivo reale o il simulatore.

use h6022_usb::{AcquisitionConfig, Calibration};

use crate::types::CoreResult;

/// Blocco di campioni in volt all'ingresso dell'oscilloscopio (sonda esclusa).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoltBlock {
    /// Indice del primo campione dall'ultima `configure`.
    pub first_sample: u64,
    pub ch1: Vec<f32>,
    /// Vuoto in modalità a un canale.
    pub ch2: Vec<f32>,
}

pub trait SampleSource: Send {
    /// Applica la configurazione e riavvia l'acquisizione. Gli indici ripartono da 0.
    fn configure(&mut self, config: &AcquisitionConfig) -> CoreResult<()>;

    /// Blocca fino al prossimo blocco. Riusa i buffer di `out`.
    fn read(&mut self, out: &mut VoltBlock) -> CoreResult<()>;

    fn calibration(&self) -> Calibration;

    /// Ferma l'acquisizione.
    fn stop(&mut self) -> CoreResult<()>;
}
