//! Conversione da forma d'onda analogica a livelli logici.

use h6022_dsp::{Waveform, measure};

/// Isteresi minima: il passo dell'ADC sulla portata ±5 V è circa 40 mV.
const MIN_HYSTERESIS: f32 = 0.05;
/// Escursione minima di un segnale logico con soglia automatica.
const MIN_SWING: f32 = 0.2;

/// Segnale logico come elenco di transizioni.
#[derive(Debug, Clone, PartialEq)]
pub struct LogicTrace {
    pub sample_rate: f64,
    pub t0: f64,
    /// Numero di campioni della forma d'onda di origine.
    pub len: usize,
    /// Livello al campione 0.
    pub initial: bool,
    /// Indici dei campioni in cui il livello cambia, crescenti. Il livello si alterna
    /// a ogni transizione, a partire da `!initial`.
    pub transitions: Vec<usize>,
    /// Soglie usate, in volt.
    pub threshold_low: f32,
    pub threshold_high: f32,
}

/// Soglia a metà tra `vbase` e `vtop` (da `measure`), con isteresi del 10 % di `vamp`
/// e almeno `MIN_HYSTERESIS`. Con `threshold` esplicita usa quella, con la stessa isteresi.
/// Con soglia automatica, un'escursione sotto `MIN_SWING` è rumore: nessuna transizione.
pub fn digitize(wave: &Waveform, threshold: Option<f32>) -> LogicTrace {
    let m = measure(wave);
    let mid = threshold.unwrap_or((m.vtop + m.vbase) / 2.0);
    let flat = threshold.is_none() && m.vpp < MIN_SWING;
    let half = if flat { f32::INFINITY } else { (m.vamp.abs() * 0.1).max(MIN_HYSTERESIS) / 2.0 };
    let (low, high) = (mid - half, mid + half);
    let initial = wave.samples.first().is_some_and(|&v| v > mid);
    let mut level = initial;
    let mut transitions = Vec::new();
    for (i, &v) in wave.samples.iter().enumerate() {
        if !level && v > high {
            level = true;
            transitions.push(i);
        } else if level && v < low {
            level = false;
            transitions.push(i);
        }
    }
    LogicTrace {
        sample_rate: wave.sample_rate,
        t0: wave.t0,
        len: wave.samples.len(),
        initial,
        transitions,
        threshold_low: low,
        threshold_high: high,
    }
}

impl LogicTrace {
    /// Costruisce una traccia da transizioni note, per i test.
    pub fn from_transitions(sample_rate: f64, len: usize, initial: bool, transitions: Vec<usize>) -> Self {
        LogicTrace { sample_rate, t0: 0.0, len, initial, transitions, threshold_low: 0.5, threshold_high: 0.5 }
    }

    /// Tempo in secondi del campione `idx`.
    pub fn time(&self, idx: usize) -> f64 {
        self.t0 + idx as f64 / self.sample_rate
    }

    /// Livello al campione `idx` (ricerca binaria sulle transizioni).
    pub fn level_at(&self, idx: usize) -> bool {
        let n = self.transitions.partition_point(|&t| t <= idx);
        self.initial ^ (n % 2 == 1)
    }

    /// Prima transizione con indice >= `from`.
    pub fn next_transition(&self, from: usize) -> Option<usize> {
        let k = self.transitions.partition_point(|&t| t < from);
        self.transitions.get(k).copied()
    }

    /// Segmenti `(inizio, fine_esclusa, livello)` che coprono `0..len`.
    pub fn runs(&self) -> Vec<(usize, usize, bool)> {
        let mut out = Vec::with_capacity(self.transitions.len() + 1);
        let mut start = 0;
        let mut level = self.initial;
        for &t in &self.transitions {
            if t > start {
                out.push((start, t, level));
            }
            start = t;
            level = !level;
        }
        if start < self.len {
            out.push((start, self.len, level));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_wave_gives_alternating_transitions() {
        let samples: Vec<f32> = (0..100).map(|i| if (i / 10) % 2 == 1 { 3.0 } else { 0.0 }).collect();
        let t = digitize(&Waveform { samples, sample_rate: 1000.0, t0: 0.0 }, None);
        assert!(!t.initial);
        assert_eq!(t.transitions, vec![10, 20, 30, 40, 50, 60, 70, 80, 90]);
        assert!(t.level_at(15) && !t.level_at(25));
        assert_eq!(t.runs()[1], (10, 20, true));
    }

    #[test]
    fn quantization_noise_has_no_transitions() {
        // Rumore di ±1 passo ADC (40 mV) attorno a zero, come un ingresso scollegato.
        let samples: Vec<f32> = (0..200).map(|i| [0.0, 0.04, -0.04, 0.0][i % 4]).collect();
        let t = digitize(&Waveform { samples, sample_rate: 1000.0, t0: 0.0 }, None);
        assert!(t.transitions.is_empty());
    }

    #[test]
    fn flat_signal_has_no_transitions() {
        let t = digitize(&Waveform { samples: vec![1.0; 50], sample_rate: 1.0, t0: 0.0 }, None);
        assert!(t.transitions.is_empty());
        assert_eq!(t.runs(), vec![(0, 50, t.initial)]);
    }
}
