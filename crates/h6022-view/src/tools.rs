//! Strumenti applicati al display: filtro su un canale e traccia matematica M (serde camelCase).

use h6022_core::Frame;
use h6022_dsp::Waveform;
use h6022_dsp::filter::{FilterKind, apply};
use h6022_dsp::math::{BinaryOp, binary, derivative, integrate, scale};
use serde::{Deserialize, Serialize};

/// Frequenze di taglio ammesse dal filtro, in Hz.
pub const CUTOFF_MIN_HZ: f64 = 10.0;
pub const CUTOFF_MAX_HZ: f64 = 10e6;
/// Picco della traccia M normalizzata (derivata e integrale), in divisioni.
pub const NORMALIZED_PEAK_DIV: f32 = 2.0;
/// Fattore di qualità del filtro Butterworth.
const FILTER_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MathOp {
    #[default]
    Off,
    /// CH1 + CH2, in volt.
    Add,
    /// CH1 − CH2, in volt.
    Sub,
    /// CH1 × CH2, in V² mostrati come volt sulla scala di CH1.
    Mul,
    /// Derivata di CH1, normalizzata a ±NORMALIZED_PEAK_DIV divisioni di CH1.
    Derivative,
    /// Integrale di CH1 senza la media, normalizzato a ±NORMALIZED_PEAK_DIV divisioni di CH1.
    Integral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterType {
    #[default]
    None,
    LowPass,
    HighPass,
    /// Passa-banda centrato su `cutoff_hz`.
    BandPass,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSettings {
    /// 0 = CH1, 1 = CH2.
    pub channel: usize,
    pub kind: FilterType,
    pub cutoff_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSettings {
    pub math: MathOp,
    pub filter: FilterSettings,
}

impl Default for ToolSettings {
    fn default() -> Self {
        ToolSettings {
            math: MathOp::Off,
            filter: FilterSettings { channel: 0, kind: FilterType::None, cutoff_hz: 10e3 },
        }
    }
}

impl ToolSettings {
    /// Errore in italiano se `filter.channel` > 1 o se `cutoff_hz` non è finito
    /// o è fuori da `CUTOFF_MIN_HZ..=CUTOFF_MAX_HZ`.
    pub fn validate(&self) -> Result<(), String> {
        if self.filter.channel > 1 {
            return Err("canale del filtro non valido".to_string());
        }
        let cutoff = self.filter.cutoff_hz;
        if !cutoff.is_finite() || !(CUTOFF_MIN_HZ..=CUTOFF_MAX_HZ).contains(&cutoff) {
            return Err("frequenza di taglio fuori intervallo (10 Hz – 10 MHz)".to_string());
        }
        Ok(())
    }
}

/// Copia di `frame` con il filtro applicato al canale scelto (Butterworth, q = 0.7071).
/// Senza filtro, o con il canale assente, restituisce una copia invariata.
pub fn filtered(frame: &Frame, filter: &FilterSettings) -> Frame {
    let kind = match filter.kind {
        FilterType::None => return frame.clone(),
        FilterType::LowPass => FilterKind::LowPass,
        FilterType::HighPass => FilterKind::HighPass,
        FilterType::BandPass => FilterKind::BandPass,
    };
    let mut out = frame.clone();
    match filter.channel {
        0 => out.ch1 = apply(&frame.ch1, kind, filter.cutoff_hz, FILTER_Q),
        1 => out.ch2 = frame.ch2.as_ref().map(|ch2| apply(ch2, kind, filter.cutoff_hz, FILTER_Q)),
        _ => {}
    }
    out
}

/// Traccia M calcolata da `frame`. `None` con `MathOp::Off`, oppure se l'operazione
/// richiede CH2 e il frame non lo ha. `ch1_volts_per_div` serve a normalizzare
/// derivata e integrale; con un valore non positivo il risultato resta in unità native.
pub fn math_trace(frame: &Frame, op: MathOp, ch1_volts_per_div: f32) -> Option<Waveform> {
    let ch1 = &frame.ch1;
    match op {
        MathOp::Off => None,
        MathOp::Add | MathOp::Sub | MathOp::Mul => {
            let ch2 = frame.ch2.as_ref()?;
            let binary_op = match op {
                MathOp::Add => BinaryOp::Add,
                MathOp::Sub => BinaryOp::Sub,
                _ => BinaryOp::Mul,
            };
            Some(binary(ch1, ch2, binary_op))
        }
        MathOp::Derivative => Some(normalizza(derivative(ch1), ch1_volts_per_div)),
        MathOp::Integral => {
            let integrale = integrate(ch1);
            let media = media(&integrale.samples);
            let senza_media = scale(&integrale, 1.0, -media);
            Some(normalizza(senza_media, ch1_volts_per_div))
        }
    }
}

/// Media dei campioni, 0 se la lista è vuota.
fn media(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&v| v as f64).sum::<f64>() / samples.len() as f64) as f32
}

/// Scala `wave` perché il massimo assoluto valga NORMALIZED_PEAK_DIV × `volts_per_div`.
/// Con `volts_per_div` non positivo, o con un segnale tutto zero, restituisce `wave` invariata.
fn normalizza(wave: Waveform, volts_per_div: f32) -> Waveform {
    if volts_per_div.is_nan() || volts_per_div <= 0.0 {
        return wave;
    }
    let picco = wave.samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if !picco.is_finite() || picco <= 0.0 {
        return wave;
    }
    scale(&wave, NORMALIZED_PEAK_DIV * volts_per_div / picco, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ViewSettings;

    fn onda(samples: Vec<f32>, sample_rate: f64) -> Waveform {
        Waveform {
            samples,
            sample_rate,
            t0: 0.0,
        }
    }

    /// Frame costruito a mano, con la configurazione iniziale della vista.
    fn frame(ch1: Waveform, ch2: Option<Waveform>) -> Frame {
        Frame {
            sequence: 1,
            ch1,
            ch2,
            triggered: true,
            trigger_sample: Some(0),
            config: ViewSettings::default().initial_config(),
        }
    }

    fn rms(s: &[f32]) -> f64 {
        (s.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
    }

    #[test]
    fn somma_di_due_costanti_e_lunghezza_minore() {
        let f = frame(onda(vec![1.0; 4], 1000.0), Some(onda(vec![0.5; 6], 1000.0)));
        let somma = math_trace(&f, MathOp::Add, 1.0).expect("somma");
        assert_eq!(somma.samples, vec![1.5; 4]);
        assert_eq!(somma.sample_rate, 1000.0);
    }

    #[test]
    fn operazioni_che_richiedono_ch2_senza_ch2_danno_none() {
        let f = frame(onda(vec![1.0; 4], 1000.0), None);
        assert!(math_trace(&f, MathOp::Add, 1.0).is_none());
        assert!(math_trace(&f, MathOp::Off, 1.0).is_none());
    }

    #[test]
    fn derivata_di_rampa_ha_picco_due_divisioni() {
        let rampa = onda((0..100).map(|i| i as f32).collect(), 1000.0);
        let f = frame(rampa, None);
        let d = math_trace(&f, MathOp::Derivative, 0.5).expect("derivata");
        let picco = d.samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((picco - 2.0 * 0.5).abs() < 1e-6, "picco {picco}");
    }

    #[test]
    fn canale_assente_lascia_il_frame_invariato() {
        let f = frame(onda(vec![0.0, 1.0, 0.0, -1.0], 1000.0), None);
        let filtro = FilterSettings { channel: 1, kind: FilterType::LowPass, cutoff_hz: 1_000.0 };
        assert_eq!(filtered(&f, &filtro), f);
    }

    #[test]
    fn passa_basso_a_1khz_attenua_50khz() {
        let fs = 1_000_000.0;
        let samples: Vec<f32> = (0..20_000)
            .map(|i| (2.0 * std::f64::consts::PI * 50_000.0 * i as f64 / fs).sin() as f32)
            .collect();
        let f = frame(onda(samples.clone(), fs), None);
        let filtro = FilterSettings { channel: 0, kind: FilterType::LowPass, cutoff_hz: 1_000.0 };
        let out = filtered(&f, &filtro);
        // Salta il transitorio.
        let skip = 5_000;
        let gain_db = 20.0 * (rms(&out.ch1.samples[skip..]) / rms(&samples[skip..])).log10();
        assert!(gain_db < -40.0, "attenuazione {gain_db} dB");
    }

    #[test]
    fn validate_rifiuta_canale_e_frequenza_fuori_intervallo() {
        let mut s = ToolSettings::default();
        assert!(s.validate().is_ok());
        s.filter.channel = 2;
        assert_eq!(s.validate(), Err("canale del filtro non valido".to_string()));
        s.filter.channel = 0;
        s.filter.cutoff_hz = 5.0;
        assert_eq!(s.validate(), Err("frequenza di taglio fuori intervallo (10 Hz – 10 MHz)".to_string()));
        s.filter.cutoff_hz = f64::NAN;
        assert!(s.validate().is_err());
    }
}
