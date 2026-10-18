//! Riduzione di un frame in colonne (min, max) per il display e formato binario di scambio.
//!
//! Formato di `encode_display`, little endian, header di 36 byte:
//! - u64 `sequence`
//! - f64 `t_start`, f64 `t_end` (secondi relativi al trigger)
//! - f64 `sample_rate` (Hz)
//! - u8 `flags`: bit 0 = frame sincronizzato sul trigger
//! - u8 `channels`: bit 0 = CH1 presente, bit 1 = CH2 presente
//! - u16 `columns`
//!
//! Seguono, per ogni canale presente in ordine CH1, CH2: `columns` f32 dei minimi,
//! poi `columns` f32 dei massimi. Una colonna senza dati vale NaN.

use h6022_core::Frame;
use h6022_dsp::Waveform;

pub const HEADER_LEN: usize = 36;
/// Colonne massime accettate da `encode_display`.
pub const MAX_COLUMNS: usize = 8192;

/// Intervallo di tempo visibile e numero di colonne del display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub t_start: f64,
    pub t_end: f64,
    pub columns: usize,
}

/// Minimi e massimi per colonna, lunghi `Viewport::columns`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnTrace {
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

/// Riduce `wave` alle colonne di `view`. La colonna `c` copre
/// `[t_start + c·dt, t_start + (c+1)·dt)` con `dt = (t_end - t_start) / columns`.
pub fn columns(wave: &Waveform, view: &Viewport) -> ColumnTrace {
    let n = view.columns;
    let mut trace = ColumnTrace { min: vec![f32::NAN; n], max: vec![f32::NAN; n] };
    let span = view.t_end - view.t_start;
    if n == 0 || !(span > 0.0) || !(wave.sample_rate > 0.0) || wave.samples.is_empty() {
        return trace;
    }
    let dt = span / n as f64;
    let len = wave.samples.len();
    // Indice del primo campione con tempo >= t.
    let index_at = |t: f64| ((t - wave.t0) * wave.sample_rate).ceil().clamp(0.0, len as f64) as usize;
    let mut next = index_at(view.t_start);
    for c in 0..n {
        let start = next;
        let end = index_at(view.t_start + (c + 1) as f64 * dt);
        next = end;
        if start >= end {
            continue;
        }
        let (lo, hi) = wave.samples[start..end]
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        trace.min[c] = lo;
        trace.max[c] = hi;
    }
    trace
}

/// Codifica le tracce dei canali presenti e attivi in `enabled` nel formato descritto sopra.
/// `view.columns` viene limitato a `1..=MAX_COLUMNS`.
pub fn encode_display(frame: &Frame, enabled: [bool; 2], view: &Viewport) -> Vec<u8> {
    let view = Viewport { columns: view.columns.clamp(1, MAX_COLUMNS), ..*view };
    let waves: Vec<&Waveform> = [Some(&frame.ch1), frame.ch2.as_ref()]
        .into_iter()
        .zip(enabled)
        .filter_map(|(wave, on)| wave.filter(|_| on))
        .collect();
    let mask = u8::from(enabled[0]) | (u8::from(enabled[1] && frame.ch2.is_some()) << 1);
    let mut out = Vec::with_capacity(HEADER_LEN + waves.len() * view.columns * 8);
    out.extend_from_slice(&frame.sequence.to_le_bytes());
    out.extend_from_slice(&view.t_start.to_le_bytes());
    out.extend_from_slice(&view.t_end.to_le_bytes());
    out.extend_from_slice(&frame.ch1.sample_rate.to_le_bytes());
    out.push(u8::from(frame.triggered));
    out.push(mask);
    out.extend_from_slice(&(view.columns as u16).to_le_bytes());
    for wave in waves {
        let trace = columns(wave, &view);
        for v in trace.min.iter().chain(&trace.max) {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(len: usize, rate: f64, t0: f64) -> Waveform {
        Waveform { samples: (0..len).map(|i| i as f32).collect(), sample_rate: rate, t0 }
    }

    #[test]
    fn columns_split_samples_by_time() {
        let wave = ramp(100, 100.0, -0.5);
        let view = Viewport { t_start: -0.5, t_end: 0.5, columns: 10 };
        let trace = columns(&wave, &view);
        assert_eq!(trace.min[0], 0.0);
        assert_eq!(trace.max[0], 9.0);
        assert_eq!(trace.min[9], 90.0);
        assert_eq!(trace.max[9], 99.0);
    }

    #[test]
    fn columns_outside_data_are_nan() {
        let wave = ramp(10, 100.0, 0.0);
        let view = Viewport { t_start: -0.1, t_end: 0.1, columns: 4 };
        let trace = columns(&wave, &view);
        assert!(trace.min[0].is_nan() && trace.min[1].is_nan());
        assert_eq!(trace.min[2], 0.0);
        assert_eq!(trace.max[3], 9.0);
    }

    #[test]
    fn more_columns_than_samples_leaves_gaps() {
        let wave = ramp(2, 10.0, 0.0);
        let view = Viewport { t_start: 0.0, t_end: 0.2, columns: 8 };
        let trace = columns(&wave, &view);
        assert_eq!(trace.min.iter().filter(|v| !v.is_nan()).count(), 2);
    }
}
