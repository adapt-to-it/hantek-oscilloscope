//! Misure automatiche su una forma d'onda.

use crate::waveform::Waveform;

/// Le misure non calcolabili (segnale piatto, meno di due fronti) valgono `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Measurements {
    pub vmax: f32,
    pub vmin: f32,
    pub vpp: f32,
    pub vmean: f32,
    pub vrms: f32,
    /// RMS della sola componente alternata.
    pub vac_rms: f32,
    /// Livello alto stabile (moda della metà superiore dell'istogramma).
    pub vtop: f32,
    /// Livello basso stabile (moda della metà inferiore).
    pub vbase: f32,
    pub vamp: f32,
    pub frequency: Option<f64>,
    pub period: Option<f64>,
    /// Frazione 0..1 del periodo sopra il livello medio.
    pub duty_positive: Option<f64>,
    pub pulse_width_positive: Option<f64>,
    pub pulse_width_negative: Option<f64>,
    /// Tempo di salita 10–90 % tra vbase e vtop, in secondi.
    pub rise_time: Option<f64>,
    pub fall_time: Option<f64>,
    /// Sovraelongazione in percentuale di vamp.
    pub overshoot_pct: Option<f32>,
    pub preshoot_pct: Option<f32>,
    pub rising_edges: usize,
    pub falling_edges: usize,
}

/// Numero di bin dell'istogramma dei livelli.
const BINS: usize = 256;

/// Un fronte: attraversamento della soglia centrale, con l'indice del campione
/// precedente (`base`) e l'istante frazionario in campioni (`t`).
#[derive(Clone, Copy)]
struct Edge {
    rising: bool,
    base: usize,
    t: f64,
}

/// Ampiezza minima, in passi di quantizzazione, perché una forma d'onda conti come segnale.
const MIN_SIGNAL_STEPS: f32 = 4.0;

/// Passo di quantizzazione: la differenza positiva minima tra due valori distinti.
/// Con due soli valori distinti la forma d'onda è un'onda quadra ideale e il passo è zero.
/// Per campioni non quantizzati (simulatore senza ADC) è quasi zero.
fn quantum(s: &[f32]) -> f32 {
    let mut sorted: Vec<f32> = s.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_unstable_by(f32::total_cmp);
    sorted.dedup();
    if sorted.len() < 3 {
        return 0.0;
    }
    sorted.windows(2).map(|w| w[1] - w[0]).fold(f32::INFINITY, f32::min)
}

pub fn measure(wave: &Waveform) -> Measurements {
    let s = &wave.samples;
    if s.is_empty() {
        return Measurements::default();
    }
    let n = s.len() as f64;
    let (vmin, vmax) = min_max(s);
    let (mut sum, mut sum_sq) = (0.0f64, 0.0f64);
    for &v in s {
        let v = v as f64;
        sum += v;
        sum_sq += v * v;
    }
    let vmean = sum / n;
    let vrms = (sum_sq / n).sqrt();
    let vac_rms = (vrms * vrms - vmean * vmean).max(0.0).sqrt();
    let (vtop, vbase) = levels(s, vmin, vmax);
    let vamp = vtop - vbase;
    // Un'ampiezza di pochi passi dell'ADC è rumore di quantizzazione: niente fronti né tempi.
    let is_signal = vamp > 0.0 && vamp >= MIN_SIGNAL_STEPS * quantum(s);
    let edges = if is_signal { crossings(s, vtop, vbase) } else { Vec::new() };

    let mut m = Measurements {
        vmax,
        vmin,
        vpp: vmax - vmin,
        vmean: vmean as f32,
        vrms: vrms as f32,
        vac_rms: vac_rms as f32,
        vtop,
        vbase,
        vamp,
        rising_edges: edges.iter().filter(|e| e.rising).count(),
        falling_edges: edges.iter().filter(|e| !e.rising).count(),
        ..Default::default()
    };
    if is_signal {
        m.overshoot_pct = Some((vmax - vtop) / vamp * 100.0);
        m.preshoot_pct = Some((vbase - vmin) / vamp * 100.0);
    }
    if is_timed(wave) {
        let rate = wave.sample_rate;
        let rise: Vec<f64> = edges
            .iter()
            .filter(|e| e.rising)
            .map(|e| seconds(wave, e.t))
            .collect();
        let fall: Vec<f64> = edges
            .iter()
            .filter(|e| !e.rising)
            .map(|e| seconds(wave, e.t))
            .collect();
        if rise.len() >= 2 {
            let period = (rise[rise.len() - 1] - rise[0]) / (rise.len() - 1) as f64;
            m.period = Some(period);
            m.frequency = Some(1.0 / period);
        }
        let (duty, pos, neg) = cycle_stats(&rise, &fall);
        m.duty_positive = duty;
        m.pulse_width_positive = pos;
        m.pulse_width_negative = neg;

        let (lo, hi) = (vbase as f64 + 0.1 * vamp as f64, vbase as f64 + 0.9 * vamp as f64);
        let (mut rise_sum, mut rise_n) = (0.0f64, 0usize);
        let (mut fall_sum, mut fall_n) = (0.0f64, 0usize);
        for e in &edges {
            if e.rising {
                if let (Some(t_lo), Some(t_hi)) = (
                    cross_before(s, e.base, lo, true),
                    cross_after(s, e.base, hi, true),
                ) {
                    rise_sum += (t_hi - t_lo) / rate;
                    rise_n += 1;
                }
            } else if let (Some(t_hi), Some(t_lo)) = (
                cross_before(s, e.base, hi, false),
                cross_after(s, e.base, lo, false),
            ) {
                fall_sum += (t_lo - t_hi) / rate;
                fall_n += 1;
            }
        }
        if rise_n > 0 {
            m.rise_time = Some(rise_sum / rise_n as f64);
        }
        if fall_n > 0 {
            m.fall_time = Some(fall_sum / fall_n as f64);
        }
    }
    m
}

/// Sfasamento in gradi di `b` rispetto ad `a`, -180..180, dai fronti di salita. `None` se non periodici.
pub fn phase_degrees(a: &Waveform, b: &Waveform) -> Option<f64> {
    let ra = rising_times(a);
    let rb = rising_times(b);
    if ra.len() < 2 || rb.len() < 2 {
        return None;
    }
    // Media circolare: ogni fronte di `b` è riportato alla frazione del periodo di `a`
    // compresa tra il fronte di `a` che lo precede e quello successivo.
    let (mut sx, mut sy, mut count) = (0.0f64, 0.0f64, 0usize);
    for &t in &rb {
        let k = ra.partition_point(|&x| x <= t);
        if k == 0 || k == ra.len() {
            continue;
        }
        let frac = (t - ra[k - 1]) / (ra[k] - ra[k - 1]);
        let angle = frac * std::f64::consts::TAU;
        sx += angle.cos();
        sy += angle.sin();
        count += 1;
    }
    if count == 0 {
        return None;
    }
    Some(sy.atan2(sx).to_degrees())
}

/// Ritardo in secondi tra il primo fronte di salita di `a` e il successivo di `b`.
pub fn delay_seconds(a: &Waveform, b: &Waveform) -> Option<f64> {
    let t_a = *rising_times(a).first()?;
    rising_times(b)
        .into_iter()
        .find(|&t| t > t_a)
        .map(|t| t - t_a)
}

fn is_timed(wave: &Waveform) -> bool {
    wave.sample_rate.is_finite() && wave.sample_rate > 0.0
}

fn seconds(wave: &Waveform, idx: f64) -> f64 {
    wave.t0 + idx / wave.sample_rate
}

/// Istanti in secondi dei fronti di salita. Vuoto se la frequenza di campionamento non è valida.
fn rising_times(wave: &Waveform) -> Vec<f64> {
    let s = &wave.samples;
    if s.is_empty() || !is_timed(wave) {
        return Vec::new();
    }
    let (vmin, vmax) = min_max(s);
    let (vtop, vbase) = levels(s, vmin, vmax);
    crossings(s, vtop, vbase)
        .iter()
        .filter(|e| e.rising)
        .map(|e| seconds(wave, e.t))
        .collect()
}

fn min_max(s: &[f32]) -> (f32, f32) {
    s.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| {
        (lo.min(v), hi.max(v))
    })
}

/// Livelli alto e basso: moda della metà superiore e inferiore dell'istogramma a 256 bin.
/// Se l'ampiezza è < 2 % di vpp o < 1 mV, usa vmax e vmin.
fn levels(s: &[f32], vmin: f32, vmax: f32) -> (f32, f32) {
    let vpp = vmax - vmin;
    if vpp.is_nan() || vpp <= 0.0 {
        return (vmax, vmin);
    }
    let scale = BINS as f64 / vpp as f64;
    let mut hist = [0usize; BINS];
    for &v in s {
        let i = (((v - vmin) as f64) * scale) as usize;
        hist[i.min(BINS - 1)] += 1;
    }
    let half = BINS / 2;
    let mode = |range: std::ops::Range<usize>| {
        range
            .clone()
            .fold(range.start, |best, i| if hist[i] > hist[best] { i } else { best })
    };
    let center = |i: usize| vmin + (i as f32 + 0.5) * vpp / BINS as f32;
    let top = center(mode(half..BINS));
    let base = center(mode(0..half));
    let vamp = top - base;
    if vamp < 0.02 * vpp || vamp < 0.001 {
        (vmax, vmin)
    } else {
        (top, base)
    }
}

/// Fronti sulla soglia centrale `(vtop + vbase) / 2`, con isteresi del 10 % di vamp.
fn crossings(s: &[f32], vtop: f32, vbase: f32) -> Vec<Edge> {
    let mid = (vtop as f64 + vbase as f64) / 2.0;
    let hyst = 0.1 * (vtop as f64 - vbase as f64);
    let mut out = Vec::new();
    let mut high = (s[0] as f64) > mid;
    let mut pending: Option<Edge> = None;
    for i in 1..s.len() {
        let p = s[i - 1] as f64;
        let c = s[i] as f64;
        if high {
            if p >= mid && c < mid {
                pending = Some(Edge { rising: false, base: i - 1, t: (i - 1) as f64 + (mid - p) / (c - p) });
            }
            if c <= mid - hyst
                && let Some(e) = pending.take() {
                    out.push(e);
                    high = false;
                }
        } else {
            if p <= mid && c > mid {
                pending = Some(Edge { rising: true, base: i - 1, t: (i - 1) as f64 + (mid - p) / (c - p) });
            }
            if c >= mid + hyst
                && let Some(e) = pending.take() {
                    out.push(e);
                    high = true;
                }
        }
    }
    out
}

/// Vero se il campione `v` è oltre `level` nel verso indicato (sopra se `up`, sotto altrimenti).
fn hits(v: f64, level: f64, up: bool) -> bool {
    if up { v >= level } else { v <= level }
}

/// Ultimo istante prima di `base` in cui il segnale attraversa `level` nel verso `up`.
fn cross_before(s: &[f32], base: usize, level: f64, up: bool) -> Option<f64> {
    let j = (0..=base).rev().find(|&j| hits(s[j] as f64, level, !up))?;
    let (a, b) = (s[j] as f64, s[j + 1] as f64);
    if hits(b, level, !up) {
        return None;
    }
    Some(j as f64 + (level - a) / (b - a))
}

/// Primo istante dopo `base` in cui il segnale attraversa `level` nel verso `up`.
fn cross_after(s: &[f32], base: usize, level: f64, up: bool) -> Option<f64> {
    let k = (base + 1..s.len()).find(|&k| hits(s[k] as f64, level, up))?;
    let (a, b) = (s[k - 1] as f64, s[k] as f64);
    Some((k - 1) as f64 + (level - a) / (b - a))
}

/// Medie sui cicli completi (fronte di salita, fronte di discesa, fronte di salita successivo).
fn cycle_stats(rise: &[f64], fall: &[f64]) -> (Option<f64>, Option<f64>, Option<f64>) {
    let (mut duty, mut pos, mut neg) = (0.0f64, 0.0f64, 0.0f64);
    let mut count = 0usize;
    let mut fi = 0usize;
    for w in rise.windows(2) {
        let (r0, r1) = (w[0], w[1]);
        while fi < fall.len() && fall[fi] <= r0 {
            fi += 1;
        }
        if fi < fall.len() && fall[fi] < r1 {
            let f = fall[fi];
            duty += (f - r0) / (r1 - r0);
            pos += f - r0;
            neg += r1 - f;
            count += 1;
        }
    }
    if count == 0 {
        return (None, None, None);
    }
    let c = count as f64;
    (Some(duty / c), Some(pos / c), Some(neg / c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn onda(samples: Vec<f32>, sample_rate: f64) -> Waveform {
        Waveform {
            samples,
            sample_rate,
            t0: 0.0,
        }
    }

    /// Quadra 0–3 V con duty 0,5.
    fn quadra(n: usize, freq: f64, fs: f64) -> Waveform {
        let samples = (0..n)
            .map(|i| {
                let fase = (i as f64 * freq / fs).fract();
                if fase < 0.5 { 3.0 } else { 0.0 }
            })
            .collect();
        onda(samples, fs)
    }

    /// Sinusoide di ampiezza `ampiezza` e fase iniziale `fase_deg`.
    fn sinusoide(n: usize, freq: f64, fs: f64, fase_deg: f64, ampiezza: f32) -> Waveform {
        let samples = (0..n)
            .map(|i| {
                let theta = std::f64::consts::TAU * freq * i as f64 / fs + fase_deg.to_radians();
                (ampiezza as f64 * theta.sin()) as f32
            })
            .collect();
        onda(samples, fs)
    }

    #[test]
    fn forma_vuota_e_tutta_zero() {
        assert_eq!(measure(&onda(Vec::new(), 1e6)), Measurements::default());
    }

    #[test]
    fn quadra_frequenza_duty_e_livelli() {
        let m = measure(&quadra(100_000, 11_000.0, 1_000_000.0));
        let f = m.frequency.expect("frequenza");
        assert!((f - 11_000.0).abs() / 11_000.0 < 0.001, "frequenza {f}");
        let duty = m.duty_positive.expect("duty");
        assert!((duty - 0.5).abs() < 0.02, "duty {duty}");
        assert!((m.vtop - 3.0).abs() < 0.05, "vtop {}", m.vtop);
        assert!(m.vbase.abs() < 0.05, "vbase {}", m.vbase);
    }

    #[test]
    fn sinusoide_vrms_e_componente_alternata() {
        let m = measure(&sinusoide(100_000, 1_000.0, 1_000_000.0, 0.0, 1.0));
        assert!((m.vrms - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.005, "vrms {}", m.vrms);
        assert!((m.vac_rms - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.005, "vac_rms {}", m.vac_rms);
        assert!(m.vmean.abs() < 0.001, "vmean {}", m.vmean);
    }

    #[test]
    fn segnale_piatto_senza_frequenza_ne_overshoot() {
        let m = measure(&onda(vec![1.0; 1000], 1e6));
        assert_eq!(m.frequency, None);
        assert_eq!(m.period, None);
        assert_eq!(m.overshoot_pct, None);
        assert_eq!(m.vrms, 1.0);
        assert_eq!(m.vac_rms, 0.0);
    }

    #[test]
    fn un_solo_fronte_non_da_frequenza() {
        let m = measure(&onda(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 1e6));
        assert_eq!(m.rising_edges, 1);
        assert_eq!(m.frequency, None);
        assert_eq!(m.period, None);
    }

    #[test]
    fn fase_di_90_gradi() {
        let a = sinusoide(10_000, 1_000.0, 1_000_000.0, 0.0, 1.0);
        let b = sinusoide(10_000, 1_000.0, 1_000_000.0, -90.0, 1.0);
        let fase = phase_degrees(&a, &b).expect("fase");
        assert!((fase - 90.0).abs() < 2.0, "fase {fase}");
        let inversa = phase_degrees(&b, &a).expect("fase inversa");
        assert!((inversa + 90.0).abs() < 2.0, "fase inversa {inversa}");
    }

    #[test]
    fn ritardo_tra_fronti_di_salita() {
        let a = sinusoide(10_000, 1_000.0, 1_000_000.0, 0.0, 1.0);
        let b = sinusoide(10_000, 1_000.0, 1_000_000.0, -90.0, 1.0);
        let ritardo = delay_seconds(&a, &b).expect("ritardo");
        assert!((ritardo - 250e-6).abs() < 1e-6, "ritardo {ritardo}");
    }

    #[test]
    fn quantization_noise_has_no_edges() {
        // Rumore di ±1 passo da 40 mV attorno a -40 mV, come un ingresso scollegato.
        let samples: Vec<f32> = (0..16000)
            .map(|i| -0.04 + 0.04 * [0.0, 1.0, -1.0, 0.0, 1.0][(i * 7 + i / 3) % 5])
            .collect();
        let m = measure(&onda(samples, 16e6));
        assert_eq!(m.rising_edges, 0);
        assert_eq!(m.frequency, None);
        assert_eq!(m.duty_positive, None);
        assert_eq!(m.overshoot_pct, None);
    }
}
