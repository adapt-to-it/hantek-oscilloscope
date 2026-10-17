//! Spettro con FFT.

use rustfft::{num_complex::Complex, FftPlanner};

use crate::waveform::Waveform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Window {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
    FlatTop,
}

/// Spettro a un solo lato.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spectrum {
    /// Larghezza di un bin in Hz.
    pub bin_hz: f64,
    /// Ampiezza RMS in volt per bin, da 0 Hz a Nyquist.
    pub vrms: Vec<f32>,
}

/// Picco spettrale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub frequency: f64,
    pub vrms: f32,
}

/// Coefficienti a0..a4 della serie di coseni: w(i) = a0 - a1 cos + a2 cos2 - a3 cos3 + a4 cos4.
const HANN: &[f64] = &[0.5, 0.5];
const HAMMING: &[f64] = &[0.54, 0.46];
const BLACKMAN_HARRIS: &[f64] = &[0.35875, 0.48829, 0.14128, 0.01168];
const FLAT_TOP: &[f64] = &[0.21557895, 0.41663158, 0.277263158, 0.083578947, 0.006947368];

/// Valore della serie di coseni con segni alternati alla posizione `i` di una finestra di lunghezza `n`.
fn cosine_sum(a: &[f64], i: usize, n: usize) -> f64 {
    let theta = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
    a.iter()
        .enumerate()
        .map(|(k, &ak)| {
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            sign * ak * (k as f64 * theta).cos()
        })
        .sum()
}

impl Window {
    pub fn all() -> [Window; 5] {
        [
            Window::Rectangular,
            Window::Hann,
            Window::Hamming,
            Window::BlackmanHarris,
            Window::FlatTop,
        ]
    }

    /// Coefficienti della finestra di lunghezza `n`.
    pub fn coefficients(self, n: usize) -> Vec<f32> {
        // Finestra periodica (denominatore n), come si usa con la FFT.
        (0..n)
            .map(|i| {
                let w = match self {
                    Window::Rectangular => 1.0,
                    Window::Hann => cosine_sum(HANN, i, n),
                    Window::Hamming => cosine_sum(HAMMING, i, n),
                    Window::BlackmanHarris => cosine_sum(BLACKMAN_HARRIS, i, n),
                    Window::FlatTop => cosine_sum(FLAT_TOP, i, n),
                };
                w as f32
            })
            .collect()
    }
}

/// Usa la potenza di 2 più grande non superiore al numero di campioni.
pub fn spectrum(wave: &Waveform, window: Window) -> Spectrum {
    let len = wave.samples.len();
    if len < 2 {
        return Spectrum::default();
    }
    let n = 1usize << (usize::BITS - 1 - len.leading_zeros());
    let x = &wave.samples[..n];

    // Rimozione della media prima della finestra.
    let mean = x.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let w = window.coefficients(n);
    // Guadagno coerente della finestra: media dei coefficienti.
    let coherent_gain = w.iter().map(|&v| v as f64).sum::<f64>() / n as f64;

    let mut buf: Vec<Complex<f32>> = x
        .iter()
        .zip(&w)
        .map(|(&v, &wi)| Complex::new(((v as f64 - mean) as f32) * wi, 0.0))
        .collect();
    let mut planner = FftPlanner::<f32>::new();
    planner.plan_fft_forward(n).process(&mut buf);

    // Una sinusoide di picco A in un bin dà |X| = A/2 · n · guadagno coerente.
    // Ampiezza RMS = A/√2, quindi il fattore √2 vale per i bin interni, non per DC e Nyquist.
    let scale = 1.0 / (n as f64 * coherent_gain);
    let half = n / 2;
    let vrms = (0..=half)
        .map(|k| {
            let mag = buf[k].norm() as f64 * scale;
            let v = if k == 0 || k == half {
                mag
            } else {
                mag * std::f64::consts::SQRT_2
            };
            v as f32
        })
        .collect();

    Spectrum {
        bin_hz: wave.sample_rate / n as f64,
        vrms,
    }
}

impl Spectrum {
    /// Ampiezza in dBV (0 dBV = 1 Vrms), con minimo -200.
    pub fn dbv(&self) -> Vec<f32> {
        self.vrms
            .iter()
            .map(|&v| (20.0 * v.log10()).max(-200.0))
            .collect()
    }

    /// I `n` picchi più alti, in ordine decrescente di ampiezza, con frequenza interpolata
    /// e ampiezza del bin massimo.
    pub fn peaks(&self, n: usize) -> Vec<Peak> {
        let m = &self.vrms;
        if n == 0 || m.len() < 3 {
            return Vec::new();
        }

        // Candidati: (indice del bin, offset parabolico in bin, ampiezza interpolata).
        let mut candidates: Vec<(usize, f64, f64)> = Vec::new();
        for k in 1..m.len() - 1 {
            let (y0, y1, y2) = (m[k - 1] as f64, m[k] as f64, m[k + 1] as f64);
            if !(y1 > y0 && y1 >= y2) {
                continue;
            }
            // L'interpolazione parabolica dà solo la frequenza; l'ampiezza è quella del bin massimo.
            let denom = y0 - 2.0 * y1 + y2;
            let offset = if denom.abs() < f64::EPSILON {
                0.0
            } else {
                (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
            };
            candidates.push((k, offset, y1));
        }

        // Dal più alto al più basso; un picco a meno di 3 bin da uno già tenuto viene scartato.
        candidates.sort_by(|a, b| b.2.total_cmp(&a.2));
        let mut kept: Vec<(usize, f64, f64)> = Vec::new();
        for c in candidates {
            if kept.iter().any(|q| q.0.abs_diff(c.0) < 3) {
                continue;
            }
            kept.push(c);
            if kept.len() == n {
                break;
            }
        }

        kept.into_iter()
            .map(|(k, offset, height)| Peak {
                frequency: (k as f64 + offset) * self.bin_hz,
                vrms: height as f32,
            })
            .collect()
    }

    /// Distorsione armonica totale in percentuale, armoniche 2..=10 della fondamentale.
    pub fn thd_pct(&self, fundamental_hz: f64) -> Option<f64> {
        if self.vrms.len() < 2 || !(fundamental_hz > 0.0) {
            return None;
        }
        let nyquist = (self.vrms.len() - 1) as f64 * self.bin_hz;
        if fundamental_hz > nyquist {
            return None;
        }
        let v1 = self.amplitude_near(fundamental_hz);
        if v1 == 0.0 {
            return None;
        }
        // Le armoniche oltre Nyquist non esistono nello spettro e non entrano nel calcolo.
        let sum_sq: f64 = (2..=10)
            .map(|h| h as f64 * fundamental_hz)
            .filter(|&f| f <= nyquist)
            .map(|f| self.amplitude_near(f).powi(2))
            .sum();
        Some(sum_sq.sqrt() / v1 * 100.0)
    }

    /// Massimo ampiezza nei bin entro ±2 dalla frequenza `hz`.
    fn amplitude_near(&self, hz: f64) -> f64 {
        let centre = (hz / self.bin_hz).round() as usize;
        let lo = centre.saturating_sub(2);
        let hi = (centre + 2).min(self.vrms.len() - 1);
        (lo..=hi)
            .map(|k| self.vrms[k] as f64)
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 1_000_000.0;
    const N: usize = 65_536;

    fn sine(f: f64) -> Waveform {
        let samples = (0..N)
            .map(|i| (2.0 * std::f64::consts::PI * f * i as f64 / FS).sin() as f32)
            .collect();
        Waveform {
            samples,
            sample_rate: FS,
            t0: 0.0,
        }
    }

    fn square(f: f64) -> Waveform {
        let samples = (0..N)
            .map(|i| {
                let phase = (f * i as f64 / FS).fract();
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();
        Waveform {
            samples,
            sample_rate: FS,
            t0: 0.0,
        }
    }

    #[test]
    fn flattop_sine_peak_is_calibrated() {
        let s = spectrum(&sine(10_000.0), Window::FlatTop);
        let peak = s.peaks(1)[0];
        assert!(
            (peak.vrms - 0.7071).abs() < 0.005,
            "vrms {}",
            peak.vrms
        );
        assert!(
            (peak.frequency - 10_000.0).abs() < 20.0,
            "frequenza {}",
            peak.frequency
        );
    }

    #[test]
    fn centred_bin_is_calibrated_for_every_window() {
        // Bin 100 esatto: la sinusoide cade al centro del bin con ogni finestra.
        let wave = sine(100.0 * FS / N as f64);
        for window in Window::all() {
            let s = spectrum(&wave, window);
            assert!(
                (s.vrms[100] - 0.70711).abs() < 0.001,
                "{window:?}: {}",
                s.vrms[100]
            );
        }
    }

    #[test]
    fn square_thd_is_between_40_and_50_percent() {
        let s = spectrum(&square(10_000.0), Window::FlatTop);
        let thd = s.thd_pct(10_000.0).expect("THD presente");
        assert!(thd > 40.0 && thd < 50.0, "THD {thd}");
    }

    #[test]
    fn thd_is_none_above_nyquist_or_zero_fundamental() {
        let s = spectrum(&sine(10_000.0), Window::FlatTop);
        assert_eq!(s.thd_pct(FS), None);
        assert_eq!(s.thd_pct(0.0), None);
    }

    #[test]
    fn flattop_coefficients_sum_to_a0_times_n() {
        let w = Window::FlatTop.coefficients(N);
        let mean = w.iter().map(|&v| v as f64).sum::<f64>() / N as f64;
        assert!((mean - 0.21557895).abs() < 1e-6, "media {mean}");
    }

    #[test]
    fn dbv_has_floor_at_minus_200() {
        let s = Spectrum {
            bin_hz: 1.0,
            vrms: vec![0.0, 1.0],
        };
        assert_eq!(s.dbv(), vec![-200.0, 0.0]);
    }
}
