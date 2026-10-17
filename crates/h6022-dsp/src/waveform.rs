/// Forma d'onda campionata in volt.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Waveform {
    pub samples: Vec<f32>,
    /// Campioni al secondo.
    pub sample_rate: f64,
    /// Tempo del primo campione in secondi, relativo al punto di trigger (negativo = pretrigger).
    pub t0: f64,
}
