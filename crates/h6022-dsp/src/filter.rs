//! Filtri digitali biquad (Butterworth, formule RBJ) e media mobile.

use crate::waveform::Waveform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

/// Biquad con stato: filtra blocchi consecutivi senza discontinuità.
#[derive(Debug, Clone)]
pub struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    /// `q` = 0.7071 dà la risposta Butterworth.
    pub fn new(kind: FilterKind, cutoff_hz: f64, q: f64, sample_rate: f64) -> Biquad {
        // Filtro passa-tutto: uscita uguale all'ingresso.
        let passthrough = Biquad {
            b: [1.0, 0.0, 0.0],
            a: [0.0, 0.0],
            z: [0.0, 0.0],
        };
        let nyquist = sample_rate / 2.0;
        if !(cutoff_hz > 0.0 && cutoff_hz < nyquist) || !(q > 0.0) {
            return passthrough;
        }

        let w0 = 2.0 * std::f64::consts::PI * cutoff_hz / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            FilterKind::LowPass => (
                (1.0 - cos) / 2.0,
                1.0 - cos,
                (1.0 - cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterKind::HighPass => (
                (1.0 + cos) / 2.0,
                -(1.0 + cos),
                (1.0 + cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            // Guadagno 0 dB al centro (costante di picco unitario).
            FilterKind::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            FilterKind::Notch => (
                1.0,
                -2.0 * cos,
                1.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
        };

        Biquad {
            b: [b0 / a0, b1 / a0, b2 / a0],
            a: [a1 / a0, a2 / a0],
            z: [0.0, 0.0],
        }
    }

    /// Forma diretta II trasposta.
    pub fn process(&mut self, samples: &mut [f32]) {
        let [b0, b1, b2] = self.b;
        let [a1, a2] = self.a;
        let [mut z0, mut z1] = self.z;
        for s in samples.iter_mut() {
            let x = *s as f64;
            let y = b0 * x + z0;
            z0 = b1 * x - a1 * y + z1;
            z1 = b2 * x - a2 * y;
            *s = y as f32;
        }
        self.z = [z0, z1];
    }

    pub fn reset(&mut self) {
        self.z = [0.0, 0.0];
    }
}

/// Applica il filtro a una copia della forma d'onda, da stato nullo.
pub fn apply(wave: &Waveform, kind: FilterKind, cutoff_hz: f64, q: f64) -> Waveform {
    let mut filter = Biquad::new(kind, cutoff_hz, q, wave.sample_rate);
    let mut samples = wave.samples.clone();
    filter.process(&mut samples);
    Waveform {
        samples,
        sample_rate: wave.sample_rate,
        t0: wave.t0,
    }
}

/// Media mobile centrata su `width` campioni (dispari; se pari usa width + 1).
pub fn moving_average(wave: &Waveform, width: usize) -> Waveform {
    let width = if width % 2 == 0 { width + 1 } else { width };
    let half = width / 2;
    let x = &wave.samples;
    let n = x.len();
    let samples = (0..n)
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half).min(n - 1);
            let sum: f64 = x[lo..=hi].iter().map(|&v| v as f64).sum();
            (sum / (hi - lo + 1) as f64) as f32
        })
        .collect();
    Waveform {
        samples,
        sample_rate: wave.sample_rate,
        t0: wave.t0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(s: &[f32]) -> f64 {
        (s.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
    }

    #[test]
    fn lowpass_1khz_attenua_di_almeno_35_db_a_10khz() {
        let fs = 1_000_000.0;
        let n = 20_000;
        let input: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 10_000.0 * i as f64 / fs).sin() as f32)
            .collect();
        let wave = Waveform {
            samples: input.clone(),
            sample_rate: fs,
            t0: 0.0,
        };
        let out = apply(&wave, FilterKind::LowPass, 1_000.0, 0.7071);
        // Salta il transitorio; la finestra copre un numero intero di periodi (100 campioni).
        let skip = 5_000;
        let gain_db = 20.0 * (rms(&out.samples[skip..]) / rms(&input[skip..])).log10();
        assert!(gain_db < -35.0, "attenuazione {gain_db} dB");
    }

    #[test]
    fn cutoff_oltre_nyquist_e_passa_tutto() {
        let wave = Waveform {
            samples: vec![0.1, -0.4, 0.7, 0.2],
            sample_rate: 1000.0,
            t0: 0.0,
        };
        let out = apply(&wave, FilterKind::LowPass, 600.0, 0.7071);
        assert_eq!(out.samples, wave.samples);
    }

    #[test]
    fn moving_average_ai_bordi_usa_i_campioni_disponibili() {
        let wave = Waveform {
            samples: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            sample_rate: 1.0,
            t0: 0.0,
        };
        let out = moving_average(&wave, 3);
        let expected = [1.5, 2.0, 3.0, 4.0, 4.5];
        for (got, want) in out.samples.iter().zip(expected) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
    }
}
