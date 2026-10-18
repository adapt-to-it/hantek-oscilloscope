//! Misure del frame più recente per il pannello del frontend (serde camelCase).

use h6022_core::Frame;
use h6022_dsp::measure::{Measurements, delay_seconds, measure, phase_degrees};
use serde::Serialize;

/// Specchio serializzabile di `h6022_dsp::measure::Measurements`, con gli stessi campi in camelCase.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMeasures {
    pub vmax: f32,
    pub vmin: f32,
    pub vpp: f32,
    pub vmean: f32,
    pub vrms: f32,
    pub vac_rms: f32,
    pub vtop: f32,
    pub vbase: f32,
    pub vamp: f32,
    pub frequency: Option<f64>,
    pub period: Option<f64>,
    pub duty_positive: Option<f64>,
    pub pulse_width_positive: Option<f64>,
    pub pulse_width_negative: Option<f64>,
    pub rise_time: Option<f64>,
    pub fall_time: Option<f64>,
    pub overshoot_pct: Option<f32>,
    pub preshoot_pct: Option<f32>,
    pub rising_edges: usize,
    pub falling_edges: usize,
}

impl From<Measurements> for ChannelMeasures {
    fn from(m: Measurements) -> Self {
        ChannelMeasures {
            vmax: m.vmax,
            vmin: m.vmin,
            vpp: m.vpp,
            vmean: m.vmean,
            vrms: m.vrms,
            vac_rms: m.vac_rms,
            vtop: m.vtop,
            vbase: m.vbase,
            vamp: m.vamp,
            frequency: m.frequency,
            period: m.period,
            duty_positive: m.duty_positive,
            pulse_width_positive: m.pulse_width_positive,
            pulse_width_negative: m.pulse_width_negative,
            rise_time: m.rise_time,
            fall_time: m.fall_time,
            overshoot_pct: m.overshoot_pct,
            preshoot_pct: m.preshoot_pct,
            rising_edges: m.rising_edges,
            falling_edges: m.falling_edges,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasureSnapshot {
    pub sequence: u64,
    /// `None` se il canale manca nel frame o è spento nella vista.
    pub channels: [Option<ChannelMeasures>; 2],
    /// CH2 rispetto a CH1, solo con entrambi i canali presenti e attivi.
    pub phase_deg: Option<f64>,
    pub delay_s: Option<f64>,
}

/// Misure di `frame` sui canali attivi in `enabled`, con `h6022_dsp::measure`.
pub fn snapshot(frame: &Frame, enabled: [bool; 2]) -> MeasureSnapshot {
    let ch1 = if enabled[0] { Some(&frame.ch1) } else { None };
    let ch2 = if enabled[1] { frame.ch2.as_ref() } else { None };
    let channels = [
        ch1.map(|w| ChannelMeasures::from(measure(w))),
        ch2.map(|w| ChannelMeasures::from(measure(w))),
    ];
    let (phase_deg, delay_s) = match (ch1, ch2) {
        (Some(a), Some(b)) => (phase_degrees(a, b), delay_seconds(a, b)),
        _ => (None, None),
    };
    MeasureSnapshot {
        sequence: frame.sequence,
        channels,
        phase_deg,
        delay_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ViewSettings;
    use h6022_dsp::Waveform;

    fn onda(samples: Vec<f32>, sample_rate: f64) -> Waveform {
        Waveform {
            samples,
            sample_rate,
            t0: 0.0,
        }
    }

    /// Quadra 0–3 V con duty 0,5, ritardata di `ritardo` campioni.
    fn quadra(n: usize, freq: f64, fs: f64, ritardo: f64) -> Waveform {
        let samples = (0..n)
            .map(|i| {
                let fase = ((i as f64 - ritardo) * freq / fs).rem_euclid(1.0);
                if fase < 0.5 { 3.0 } else { 0.0 }
            })
            .collect();
        onda(samples, fs)
    }

    /// Frame costruito a mano, con la configurazione iniziale della vista.
    fn frame(ch1: Waveform, ch2: Option<Waveform>) -> Frame {
        Frame {
            sequence: 7,
            ch1,
            ch2,
            triggered: true,
            trigger_sample: Some(0),
            config: ViewSettings::default().initial_config(),
        }
    }

    #[test]
    fn ch2_assente_da_none() {
        let f = frame(quadra(10_000, 1_000.0, 1_000_000.0, 0.0), None);
        let s = snapshot(&f, [true, true]);
        assert_eq!(s.sequence, 7);
        assert!(s.channels[0].is_some());
        assert_eq!(s.channels[1], None);
        assert_eq!(s.phase_deg, None);
        assert_eq!(s.delay_s, None);
        let f1 = s.channels[0].expect("CH1").frequency.expect("frequenza");
        assert!((f1 - 1_000.0).abs() / 1_000.0 < 0.001, "frequenza {f1}");
    }

    #[test]
    fn canale_spento_da_none_anche_se_presente() {
        let f = frame(
            quadra(10_000, 1_000.0, 1_000_000.0, 0.0),
            Some(quadra(10_000, 1_000.0, 1_000_000.0, 250.0)),
        );
        let solo_ch1 = snapshot(&f, [true, false]);
        assert!(solo_ch1.channels[0].is_some());
        assert_eq!(solo_ch1.channels[1], None);
        assert_eq!(solo_ch1.phase_deg, None);
        assert_eq!(solo_ch1.delay_s, None);

        let solo_ch2 = snapshot(&f, [false, true]);
        assert_eq!(solo_ch2.channels[0], None);
        assert!(solo_ch2.channels[1].is_some());
        assert_eq!(solo_ch2.phase_deg, None);
        assert_eq!(solo_ch2.delay_s, None);
    }

    #[test]
    fn fase_di_quadre_sfasate_di_un_quarto() {
        // Periodo di 1000 campioni: CH2 ritardata di 250 campioni, cioè un quarto di periodo.
        let f = frame(
            quadra(10_000, 1_000.0, 1_000_000.0, 0.0),
            Some(quadra(10_000, 1_000.0, 1_000_000.0, 250.0)),
        );
        let s = snapshot(&f, [true, true]);
        let fase = s.phase_deg.expect("fase");
        assert!((fase - 90.0).abs() < 2.0, "fase {fase}");
        let ritardo = s.delay_s.expect("ritardo");
        assert!((ritardo - 250e-6).abs() < 2e-6, "ritardo {ritardo}");
        assert!(s.channels[0].is_some() && s.channels[1].is_some());
    }
}
