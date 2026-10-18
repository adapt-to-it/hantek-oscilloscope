//! Ricerca di una forma d'onda campione dentro una registrazione.

use h6022_dsp::Waveform;
use rustfft::{FftPlanner, num_complex::Complex};

/// Varianza considerata nulla quando è sotto questa frazione dell'energia del tratto.
const FLAT_REL: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    /// Indice del primo campione nella registrazione.
    pub index: usize,
    pub time_s: f64,
    /// Correlazione normalizzata -1..1.
    pub score: f32,
}

/// Correlazione incrociata normalizzata (media e varianza locali) calcolata con FFT.
/// Restituisce i massimi locali con score >= `min_score`, separati da almeno metà della
/// lunghezza del campione, in ordine di tempo, al più `max_matches`.
/// Le due forme d'onda devono avere la stessa frequenza; altrimenti, o con campione vuoto
/// o più lungo della registrazione, restituisce un vettore vuoto.
pub fn find(haystack: &Waveform, template: &Waveform, min_score: f32, max_matches: usize) -> Vec<Match> {
    let n = template.samples.len();
    let big_n = haystack.samples.len();
    if n == 0 || n > big_n || max_matches == 0 || !same_rate(haystack.sample_rate, template.sample_rate) {
        return Vec::new();
    }

    // Template centrato: Σx·t − n·mx·mt = Σx·(t − mt), perché Σ(t − mt) = 0.
    let t_mean = template.samples.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let t_c: Vec<f64> = template.samples.iter().map(|&v| v as f64 - t_mean).collect();
    let stt: f64 = t_c.iter().map(|d| d * d).sum();
    let t_energy: f64 = template.samples.iter().map(|&v| (v as f64) * (v as f64)).sum();
    if stt <= FLAT_REL * t_energy {
        return Vec::new();
    }

    // Correlazione con FFT a lunghezza potenza di 2 >= N + n, con zero padding.
    let len = (big_n + n).next_power_of_two();
    let mut planner = FftPlanner::<f64>::new();
    let forward = planner.plan_fft_forward(len);
    let inverse = planner.plan_fft_inverse(len);
    let mut x: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); len];
    for (dst, &v) in x.iter_mut().zip(&haystack.samples) {
        *dst = Complex::new(v as f64, 0.0);
    }
    let mut t: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); len];
    for (dst, &v) in t.iter_mut().zip(&t_c) {
        *dst = Complex::new(v, 0.0);
    }
    forward.process(&mut x);
    forward.process(&mut t);
    // X · conj(T) antitrasformato dà r[i] = Σ_j x[i+j]·t[j]; per i <= N − n non ci sono avvolgimenti.
    for (xi, ti) in x.iter_mut().zip(&t) {
        *xi = *xi * ti.conj();
    }
    inverse.process(&mut x);
    let norm = 1.0 / len as f64;

    // Somme cumulate di x e x² per media e varianza di ogni finestra.
    let mut sum = vec![0.0f64; big_n + 1];
    let mut sum_sq = vec![0.0f64; big_n + 1];
    for (i, &v) in haystack.samples.iter().enumerate() {
        let v = v as f64;
        sum[i + 1] = sum[i] + v;
        sum_sq[i + 1] = sum_sq[i] + v * v;
    }

    let windows = big_n - n + 1;
    let mut scores = vec![0.0f64; windows];
    for (i, score) in scores.iter_mut().enumerate() {
        let s = sum[i + n] - sum[i];
        let s2 = sum_sq[i + n] - sum_sq[i];
        let sxx = s2 - s * s / n as f64;
        // Finestra piatta: score 0.
        if sxx > FLAT_REL * s2 {
            let cov = x[i].re * norm;
            *score = (cov / (sxx * stt).sqrt()).clamp(-1.0, 1.0);
        }
    }

    // Massimi locali sopra soglia (plateau: si tiene il primo campione).
    let min_score = f64::from(min_score);
    let mut peaks: Vec<(usize, f64)> = Vec::new();
    for (i, &s) in scores.iter().enumerate() {
        let left = i == 0 || s >= scores[i - 1];
        let right = i + 1 == windows || s > scores[i + 1];
        if s > 0.0 && s >= min_score && left && right {
            peaks.push((i, s));
        }
    }

    // Dal più alto: scarta quelli a meno di n/2 da uno già preso.
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut taken: Vec<(usize, f64)> = Vec::new();
    for (i, s) in peaks {
        if taken.len() == max_matches {
            break;
        }
        if taken.iter().all(|&(j, _)| i.abs_diff(j) * 2 >= n) {
            taken.push((i, s));
        }
    }

    taken.sort_by_key(|&(i, _)| i);
    taken
        .into_iter()
        .map(|(index, score)| Match {
            index,
            time_s: haystack.t0 + index as f64 / haystack.sample_rate,
            score: score as f32,
        })
        .collect()
}

fn same_rate(a: f64, b: f64) -> bool {
    a.is_finite() && a > 0.0 && (a - b).abs() <= 1e-9 * a
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 1000.0;

    /// Impulso sagomato, asimmetrico, di 20 campioni.
    fn pulse() -> Vec<f32> {
        (0..20)
            .map(|j| {
                let x = j as f32 / 19.0;
                (std::f32::consts::PI * x).sin().powi(2) * (1.0 + 0.5 * x) * 2.0
            })
            .collect()
    }

    /// Rumore deterministico in ±0,1 V.
    fn noise(len: usize) -> Vec<f32> {
        let mut seed: u32 = 12345;
        (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 0.2
            })
            .collect()
    }

    #[test]
    fn trova_tre_copie_dell_impulso_nel_rumore() {
        let p = pulse();
        let mut hay = noise(1000);
        let starts = [100usize, 500, 800];
        for &s in &starts {
            for (j, &v) in p.iter().enumerate() {
                hay[s + j] += v;
            }
        }
        let haystack = Waveform { samples: hay, sample_rate: SR, t0: 0.0 };
        let template = Waveform { samples: p, sample_rate: SR, t0: 0.0 };

        // Con 20 campioni il rumore da solo arriva a score ~0,7: la soglia sta sopra.
        let found = find(&haystack, &template, 0.9, 10);
        assert_eq!(found.len(), 3, "trovati {found:?}");
        for (m, &s) in found.iter().zip(&starts) {
            assert!(m.index.abs_diff(s) <= 1, "indice {} atteso vicino a {s}", m.index);
            assert!(m.score > 0.9, "score {}", m.score);
        }
    }

    #[test]
    fn template_piu_lungo_della_registrazione_da_vuoto() {
        let haystack = Waveform { samples: vec![0.0, 1.0, 0.0, 1.0], sample_rate: SR, t0: 0.0 };
        let template = Waveform { samples: pulse(), sample_rate: SR, t0: 0.0 };
        assert!(find(&haystack, &template, 0.5, 10).is_empty());
    }
}
