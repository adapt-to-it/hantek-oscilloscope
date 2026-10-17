//! Decimazione per il display e interpolazione.

use std::f64::consts::PI;

/// Numero di lobi della finestra di Lanczos.
const LANCZOS_LOBES: f64 = 8.0;

/// Divide `samples` in `buckets` intervalli uguali e restituisce (min, max) di ognuno.
/// Se i campioni sono meno dei bucket, restituisce un bucket per campione.
pub fn min_max_decimate(samples: &[f32], buckets: usize) -> Vec<(f32, f32)> {
    if buckets == 0 || samples.is_empty() {
        return Vec::new();
    }
    let n = samples.len();
    let count = buckets.min(n);
    (0..count)
        .map(|i| {
            let lo = i * n / count;
            let hi = (i + 1) * n / count;
            let slice = &samples[lo..hi];
            let min = slice.iter().copied().fold(f32::INFINITY, f32::min);
            let max = slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (min, max)
        })
        .collect()
}

fn sinc(t: f64) -> f64 {
    if t == 0.0 {
        1.0
    } else {
        let x = PI * t;
        x.sin() / x
    }
}

/// Interpolazione sin(x)/x con finestra di Lanczos a 8 lobi. Restituisce
/// (len - 1) * factor + 1 campioni che passano per i campioni originali.
pub fn sinc_interpolate(samples: &[f32], factor: usize) -> Vec<f32> {
    if factor <= 1 || samples.len() < 2 {
        return samples.to_vec();
    }
    let n = samples.len();
    let out_len = (n - 1) * factor + 1;
    let a = LANCZOS_LOBES;
    (0..out_len)
        .map(|k| {
            let pos = k as f64 / factor as f64;
            let centre = pos.floor() as i64;
            let mut acc = 0.0f64;
            let mut wsum = 0.0f64;
            // Il supporto del kernel è |t| < a: con a = 8 bastano questi indici.
            for j in (centre - 7)..=(centre + 8) {
                if j < 0 || j >= n as i64 {
                    continue;
                }
                let t = pos - j as f64;
                if t.abs() >= a {
                    continue;
                }
                let w = sinc(t) * sinc(t / a);
                acc += w * samples[j as usize] as f64;
                wsum += w;
            }
            // Normalizza sui pesi effettivi: esatta ai campioni originali e ai bordi.
            if wsum != 0.0 {
                (acc / wsum) as f32
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_max_decimate_copre_tutti_i_campioni_una_volta() {
        let samples: Vec<f32> = (0..10).map(|v| v as f32).collect();
        let out = min_max_decimate(&samples, 3);
        assert_eq!(out, vec![(0.0, 2.0), (3.0, 5.0), (6.0, 9.0)]);
    }

    #[test]
    fn min_max_decimate_casi_limite() {
        let samples = [1.0f32, 5.0];
        assert!(min_max_decimate(&samples, 0).is_empty());
        assert_eq!(min_max_decimate(&samples, 8), vec![(1.0, 1.0), (5.0, 5.0)]);
    }

    #[test]
    fn sinc_interpolate_sinusoide_a_un_decimo_di_nyquist() {
        let factor = 4;
        // Un periodo ogni 20 campioni, cioè 1/10 della frequenza di Nyquist.
        let input: Vec<f32> = (0..200)
            .map(|j| (2.0 * PI * j as f64 / 20.0).sin() as f32)
            .collect();
        let out = sinc_interpolate(&input, factor);
        assert_eq!(out.len(), (input.len() - 1) * factor + 1);

        // Passa per i campioni originali.
        for (j, &x) in input.iter().enumerate() {
            assert!((out[j * factor] - x).abs() < 1e-5);
        }
        // Lontano dai bordi scosta meno dell'1 % dalla sinusoide vera.
        for k in 40..=760 {
            let pos = k as f64 / factor as f64;
            let truth = (2.0 * PI * pos / 20.0).sin();
            assert!(
                (out[k] as f64 - truth).abs() < 0.01,
                "k={k}: {} vs {truth}",
                out[k]
            );
        }
    }

    #[test]
    fn sinc_interpolate_fattore_uno_o_pochi_campioni_copia() {
        let samples = [0.25f32, -0.5, 0.75];
        assert_eq!(sinc_interpolate(&samples, 1), samples.to_vec());
        assert_eq!(sinc_interpolate(&samples, 0), samples.to_vec());
        assert_eq!(sinc_interpolate(&samples[..1], 4), vec![0.25]);
    }
}
