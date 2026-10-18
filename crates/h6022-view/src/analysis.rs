//! Spettro e punti XY del frame più recente, pronti per il frontend (serde camelCase).

use h6022_core::Frame;
use h6022_dsp::Window;
use h6022_dsp::spectrum::spectrum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowName {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
    FlatTop,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeakView {
    pub frequency: f64,
    pub dbv: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSpectrum {
    /// Ampiezza in dBV per bin dopo la riduzione, da 0 Hz a Nyquist.
    pub dbv: Vec<f32>,
    /// I 5 picchi più alti, in ordine di ampiezza decrescente.
    pub peaks: Vec<PeakView>,
    /// THD sulla fondamentale (il picco più alto), se calcolabile.
    pub thd_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectrumView {
    pub sequence: u64,
    /// Larghezza di un bin dopo la riduzione, in Hz.
    pub bin_hz: f64,
    pub nyquist_hz: f64,
    /// `None` se il canale manca o è spento.
    pub channels: [Option<ChannelSpectrum>; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XyView {
    pub sequence: u64,
    /// Volt di CH1 (asse x) e CH2 (asse y), stessa lunghezza. Vuoti se manca CH2.
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}

/// Spettro dei canali attivi con `h6022_dsp::spectrum`. Se i bin superano `max_bins`, li riduce
/// prendendo il massimo in dBV di ogni gruppo di bin consecutivi, così i picchi restano visibili.
pub fn spectrum_view(frame: &Frame, enabled: [bool; 2], window: WindowName, max_bins: usize) -> SpectrumView {
    let window = window_of(window);
    let waves = [
        enabled[0].then_some(&frame.ch1),
        if enabled[1] { frame.ch2.as_ref() } else { None },
    ];
    let mut channels: [Option<ChannelSpectrum>; 2] = [None, None];
    // Passo dei bin e Nyquist sono quelli del primo canale attivo.
    let mut bin_hz = None;
    for (i, wave) in waves.into_iter().enumerate() {
        let Some(wave) = wave else { continue };
        let full = spectrum(wave, window);
        let (group, dbv) = reduce_dbv(&full.dbv(), max_bins);
        bin_hz.get_or_insert(full.bin_hz * group as f64);
        let peaks: Vec<PeakView> = full
            .peaks(5)
            .into_iter()
            .map(|p| PeakView {
                frequency: p.frequency,
                dbv: (20.0 * p.vrms.log10()).max(-200.0),
            })
            .collect();
        let thd_pct = peaks.first().and_then(|p| full.thd_pct(p.frequency));
        channels[i] = Some(ChannelSpectrum { dbv, peaks, thd_pct });
    }
    SpectrumView {
        sequence: frame.sequence,
        bin_hz: bin_hz.unwrap_or(0.0),
        nyquist_hz: frame.ch1.sample_rate / 2.0,
        channels,
    }
}

/// Coppie (CH1, CH2) del frame, al massimo `max_points` prese a passo costante.
pub fn xy_view(frame: &Frame, max_points: usize) -> XyView {
    let Some(ch2) = frame.ch2.as_ref() else {
        return XyView {
            sequence: frame.sequence,
            x: Vec::new(),
            y: Vec::new(),
        };
    };
    let len = frame.ch1.samples.len().min(ch2.samples.len());
    let step = len.div_ceil(max_points.max(1)).max(1);
    XyView {
        sequence: frame.sequence,
        x: frame.ch1.samples[..len].iter().step_by(step).copied().collect(),
        y: ch2.samples[..len].iter().step_by(step).copied().collect(),
    }
}

fn window_of(name: WindowName) -> Window {
    match name {
        WindowName::Rectangular => Window::Rectangular,
        WindowName::Hann => Window::Hann,
        WindowName::Hamming => Window::Hamming,
        WindowName::BlackmanHarris => Window::BlackmanHarris,
        WindowName::FlatTop => Window::FlatTop,
    }
}

/// Gruppi di `ceil(n/max_bins)` bin consecutivi, ciascuno col suo massimo in dBV.
/// Ritorna la dimensione del gruppo e i valori ridotti.
fn reduce_dbv(dbv: &[f32], max_bins: usize) -> (usize, Vec<f32>) {
    let group = dbv.len().div_ceil(max_bins.max(1)).max(1);
    let reduced = dbv
        .chunks(group)
        .map(|c| c.iter().copied().fold(f32::NEG_INFINITY, f32::max))
        .collect();
    (group, reduced)
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

    /// Sinusoide di 1 V di picco.
    fn sinusoide(n: usize, freq: f64, fs: f64) -> Waveform {
        let samples = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / fs).sin() as f32)
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
    fn sinusoide_1khz_picco_entro_un_bin() {
        let fs = 1_000_000.0;
        let f = frame(sinusoide(10_000, 1_000.0, fs), None);
        let v = spectrum_view(&f, [true, false], WindowName::Hann, 10_000);
        let ch = v.channels[0].as_ref().expect("CH1");
        let picco = ch.peaks[0].frequency;
        assert!((picco - 1_000.0).abs() <= v.bin_hz, "picco {picco} Hz, bin {} Hz", v.bin_hz);
        assert_eq!(v.sequence, 7);
        assert!(v.channels[1].is_none());
    }

    #[test]
    fn riduzione_limita_i_bin_e_conserva_il_massimo() {
        let fs = 1_000_000.0;
        let f = frame(sinusoide(10_000, 1_000.0, fs), None);
        let piena = spectrum(&f.ch1, Window::Hann).dbv();
        let v = spectrum_view(&f, [true, false], WindowName::Hann, 64);
        let ch = v.channels[0].as_ref().expect("CH1");
        assert!(ch.dbv.len() <= 64, "lunghezza {}", ch.dbv.len());
        // 8192 bin utili (n = 8192), gruppi da ceil(4097/64) = 65.
        assert!((v.bin_hz - 65.0 * fs / 8192.0).abs() < 1e-6, "bin {}", v.bin_hz);
        let max_piena = piena.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let max_ridotta = ch.dbv.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(max_ridotta, max_piena);
    }

    #[test]
    fn canale_spento_o_assente_da_none() {
        let f = frame(
            sinusoide(10_000, 1_000.0, 1_000_000.0),
            Some(sinusoide(10_000, 500.0, 1_000_000.0)),
        );
        let solo_ch2 = spectrum_view(&f, [false, true], WindowName::Hann, 512);
        assert!(solo_ch2.channels[0].is_none());
        assert!(solo_ch2.channels[1].is_some());

        let spento = spectrum_view(&f, [true, false], WindowName::Hann, 512);
        assert!(spento.channels[0].is_some());
        assert!(spento.channels[1].is_none());

        let senza_ch2 = frame(sinusoide(10_000, 1_000.0, 1_000_000.0), None);
        let v = spectrum_view(&senza_ch2, [true, true], WindowName::Hann, 512);
        assert!(v.channels[1].is_none());
    }

    #[test]
    fn xy_senza_ch2_e_vuoto() {
        let f = frame(sinusoide(10_000, 1_000.0, 1_000_000.0), None);
        let v = xy_view(&f, 1_000);
        assert_eq!(v.sequence, 7);
        assert!(v.x.is_empty());
        assert!(v.y.is_empty());
    }

    #[test]
    fn xy_limitato_a_max_points() {
        let f = frame(
            sinusoide(10_000, 1_000.0, 1_000_000.0),
            Some(sinusoide(10_000, 500.0, 1_000_000.0)),
        );
        let v = xy_view(&f, 1_000);
        assert_eq!(v.x.len(), v.y.len());
        assert!(!v.x.is_empty() && v.x.len() <= 1_000, "punti {}", v.x.len());
    }
}
