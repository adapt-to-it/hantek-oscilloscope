//! Classificazione automatica del segnale e segmentazione in pacchetti.

use h6022_dsp::spectrum::spectrum;
use h6022_dsp::{Waveform, Window, measure};

use crate::{LogicTrace, digitize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalClass {
    /// Livello costante, vac_rms < 2 % del fondo scala usato o < 20 mV.
    Dc,
    /// Variazioni senza fronti netti (nessuna transizione con l'isteresi di `digitize`).
    Noise,
    /// Sinusoide o forma analogica periodica: THD < 5 % alla frequenza misurata.
    Analog,
    /// Onda rettangolare con periodo regolare (jitter < 2 %) e duty 45..55 %.
    Clock,
    /// Rettangolare regolare con altro duty, oppure duty variabile a periodo costante.
    Pwm,
    /// Gruppi di impulsi separati da pause lunghe (vedi `segment`), impulsi di durata multipla di un'unità.
    Serial,
    /// Gruppi di impulsi separati da pause, senza unità comune.
    Burst,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub class: SignalClass,
    /// 0..1, quanto i criteri sono rispettati.
    pub confidence: f32,
    /// Una frase in italiano con i numeri chiave.
    pub summary: String,
    /// Per `Serial`: baud stimato dall'impulso minimo.
    pub baud_estimate: Option<f64>,
    /// Decoder suggeriti, per esempio ["uart", "manchester"].
    pub suggested_decoders: Vec<&'static str>,
}

/// Ampiezza alternata sotto cui il segnale è considerato continuo, in volt.
const DC_VAC_RMS: f32 = 0.02;
/// THD massima, in percento, per un segnale analogico.
const THD_LIMIT_PCT: f64 = 5.0;
/// Jitter massimo dei periodi, come frazione del periodo medio.
const JITTER_LIMIT: f64 = 0.02;
/// Variazione massima del duty tra cicli, come frazione del periodo.
const DUTY_SPREAD_LIMIT: f64 = 0.02;
const CLOCK_DUTY_MIN: f64 = 0.45;
const CLOCK_DUTY_MAX: f64 = 0.55;
/// Scostamento massimo di ogni impulso da un multiplo intero dell'impulso minimo.
const SERIAL_TOL: f64 = 0.2;

/// Salita più discesa, in frazione del periodo, oltre cui la forma è analogica e non rettangolare.
const SLOW_EDGES: f64 = 0.3;

pub fn classify(wave: &Waveform) -> Classification {
    let m = measure(wave);
    let vac_mv = m.vac_rms as f64 * 1000.0;

    // 1. Livello costante.
    if m.vac_rms < DC_VAC_RMS {
        let margin = (DC_VAC_RMS - m.vac_rms) as f64 / DC_VAC_RMS as f64;
        return done(
            SignalClass::Dc,
            confidence(margin),
            format!("Livello costante, vac_rms {} mV", decimal(vac_mv, 1)),
            None,
        );
    }

    // 2. Nessuna transizione: solo rumore.
    let trace = digitize(wave, None);
    if trace.transitions.is_empty() {
        return done(
            SignalClass::Noise,
            1.0,
            format!("Rumore senza fronti, vac_rms {} mV", decimal(vac_mv, 1)),
            None,
        );
    }

    // 3. Forma analogica: THD bassa alla frequenza misurata. Se la finestra ha troppi pochi
    // periodi per la THD, decidono i fronti: una sinusoide sale e scende in circa il 60 % del periodo.
    if let (Some(f), Some(period), Some(rise), Some(fall)) = (m.frequency, m.period, m.rise_time, m.fall_time) {
        let edges = (rise + fall) / period;
        if spectrum(wave, Window::FlatTop).thd_pct(f).is_none() && edges >= SLOW_EDGES {
            return done(
                SignalClass::Analog,
                confidence(((edges - SLOW_EDGES) / SLOW_EDGES).min(1.0)),
                format!("Analogico {}, fronti lenti", frequency_text(f)),
                None,
            );
        }
    }
    if let Some(f) = m.frequency
        && let Some(thd) = spectrum(wave, Window::FlatTop).thd_pct(f)
            && thd < THD_LIMIT_PCT {
                let margin = (THD_LIMIT_PCT - thd) / THD_LIMIT_PCT;
                return done(
                    SignalClass::Analog,
                    confidence(margin),
                    format!("Analogico {}, THD {} %", frequency_text(f), decimal(thd, 1)),
                    None,
                );
            }

    // 4. Fronti di salita periodici: Clock o Pwm.
    if let Some(found) = periodic(&trace) {
        return found;
    }

    // 5. Pacchetti di impulsi: Serial se tutti multipli di un'unità comune, altrimenti Burst.
    let segs = segment(&trace, None);
    if segs.len() >= 2 {
        let impulses = impulse_lengths(&trace, &segs);
        if let Some(min) = impulses.iter().copied().min() {
            let mut worst = 0.0f64;
            let mut violating = 0usize;
            for &d in &impulses {
                let ratio = d as f64 / min as f64;
                let multiple = ratio.round().max(1.0);
                let dev = (ratio - multiple).abs() / multiple;
                worst = worst.max(dev);
                if dev > SERIAL_TOL {
                    violating += 1;
                }
            }
            if violating == 0 {
                let baud = trace.sample_rate / min as f64;
                let margin = (SERIAL_TOL - worst) / SERIAL_TOL;
                return done(
                    SignalClass::Serial,
                    confidence(margin),
                    format!("Seriale, {} pacchetti, baud stimato {}", segs.len(), decimal(baud, 0)),
                    Some(baud),
                );
            }
            let frac = violating as f64 / impulses.len() as f64;
            return done(
                SignalClass::Burst,
                confidence(frac),
                format!("Raffiche di {} pacchetti senza unità comune", segs.len()),
                None,
            );
        }
    }

    // 6. Nessun criterio applicabile.
    done(SignalClass::Other, 0.5, "Forma non riconosciuta".to_string(), None)
}

/// Clock o Pwm se i fronti di salita sono periodici (almeno due periodi) con jitter sotto soglia.
fn periodic(trace: &LogicTrace) -> Option<Classification> {
    // Fronte di salita: la transizione k porta il livello a `initial ^ (k % 2 == 0)`.
    let rises: Vec<(usize, usize)> = trace
        .transitions
        .iter()
        .enumerate()
        .filter(|(k, _)| trace.initial ^ (*k % 2 == 0))
        .map(|(k, &t)| (k, t))
        .collect();
    if rises.len() < 3 {
        return None;
    }

    let mut periods = Vec::with_capacity(rises.len() - 1);
    let mut duties = Vec::with_capacity(rises.len() - 1);
    for pair in rises.windows(2) {
        let (k, t0) = pair[0];
        let t1 = pair[1].1;
        // La transizione successiva al fronte di salita è il fronte di discesa.
        let fall = trace.transitions[k + 1];
        periods.push((t1 - t0) as f64);
        duties.push((fall - t0) as f64 / (t1 - t0) as f64);
    }

    let period = mean(&periods);
    let jitter = std_dev(&periods, period) / period;
    if jitter >= JITTER_LIMIT {
        return None;
    }

    let duty = mean(&duties);
    let (lo, hi) = duties
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &d| (lo.min(d), hi.max(d)));
    let spread = hi - lo;
    let freq = trace.sample_rate / period;
    let margin = (JITTER_LIMIT - jitter) / JITTER_LIMIT;

    let is_clock = (CLOCK_DUTY_MIN..=CLOCK_DUTY_MAX).contains(&duty) && spread < DUTY_SPREAD_LIMIT;
    if is_clock {
        Some(done(
            SignalClass::Clock,
            confidence(margin),
            format!("Clock {}, duty {} %", frequency_text(freq), decimal(duty * 100.0, 1)),
            None,
        ))
    } else {
        Some(done(
            SignalClass::Pwm,
            confidence(margin),
            format!("PWM {}, duty medio {} %", frequency_text(freq), decimal(duty * 100.0, 1)),
            None,
        ))
    }
}

/// Lunghezza in campioni di ogni impulso interno ai pacchetti (tra due transizioni del pacchetto).
fn impulse_lengths(trace: &LogicTrace, segs: &[Segment]) -> Vec<usize> {
    let index_of = |s: f64| ((s - trace.t0) * trace.sample_rate).round() as usize;
    let bounds: Vec<(usize, usize)> = segs
        .iter()
        .map(|s| (index_of(s.start_s), index_of(s.end_s)))
        .collect();
    trace
        .runs()
        .into_iter()
        .filter(|&(a, b, _)| bounds.iter().any(|&(lo, hi)| a >= lo && b <= hi))
        .map(|(a, b, _)| b - a)
        .collect()
}

/// Pacchetto: intervallo di attività tra due pause.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start_s: f64,
    pub end_s: f64,
    pub transitions: usize,
}

/// Pausa = livello costante più lungo di `min_gap_s`. Con `None`: 10 volte la mediana delle durate
/// degli impulsi. Restituisce i tratti con almeno una transizione.
pub fn segment(trace: &LogicTrace, min_gap_s: Option<f64>) -> Vec<Segment> {
    let gap_s = match min_gap_s {
        Some(g) => g,
        None => {
            let mut durations: Vec<f64> = trace
                .runs()
                .iter()
                .map(|&(a, b, _)| (b - a) as f64 / trace.sample_rate)
                .collect();
            if durations.is_empty() {
                return Vec::new();
            }
            durations.sort_by(f64::total_cmp);
            let mid = durations.len() / 2;
            let median = if durations.len() % 2 == 1 {
                durations[mid]
            } else {
                (durations[mid - 1] + durations[mid]) / 2.0
            };
            10.0 * median
        }
    };

    let tr = &trace.transitions;
    let mut out = Vec::new();
    let mut first = 0usize;
    for k in 0..tr.len() {
        let is_last = k + 1 == tr.len();
        let pause_after = !is_last && (tr[k + 1] - tr[k]) as f64 / trace.sample_rate > gap_s;
        if is_last || pause_after {
            out.push(Segment {
                start_s: trace.time(tr[first]),
                end_s: trace.time(tr[k]),
                transitions: k - first + 1,
            });
            first = k + 1;
        }
    }
    out
}

fn done(class: SignalClass, confidence: f32, summary: String, baud_estimate: Option<f64>) -> Classification {
    let suggested_decoders = match class {
        SignalClass::Clock | SignalClass::Pwm => vec!["pwm"],
        SignalClass::Serial => vec!["uart", "manchester", "onewire"],
        SignalClass::Burst => vec!["ir_nec", "ir_rc5"],
        _ => Vec::new(),
    };
    Classification { class, confidence, summary, baud_estimate, suggested_decoders }
}

/// Confidenza da margine normalizzato: 1.0 se il criterio è largamente rispettato, 0.5 sulla soglia.
fn confidence(margin: f64) -> f32 {
    (0.5 + 0.5 * margin.clamp(0.0, 1.0)) as f32
}

/// Numero con la virgola decimale, come nei riepiloghi in italiano.
fn decimal(x: f64, digits: usize) -> String {
    format!("{:.*}", digits, x).replace('.', ",")
}

fn frequency_text(hz: f64) -> String {
    if hz >= 1000.0 {
        format!("{} kHz", decimal(hz / 1000.0, 2))
    } else {
        format!("{} Hz", decimal(hz, 2))
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn std_dev(v: &[f64], mean: f64) -> f64 {
    (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / v.len() as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    const SR: f64 = 100_000.0;

    fn wave(samples: Vec<f32>) -> Waveform {
        Waveform { samples, sample_rate: SR, t0: 0.0 }
    }

    /// Quadra con periodo di `period` campioni, livello alto per `high` campioni di ogni periodo.
    fn square(period: usize, high: usize, cycles: usize) -> Waveform {
        let samples: Vec<f32> = (0..period * cycles)
            .map(|i| if i % period < high { 3.3 } else { 0.0 })
            .collect();
        wave(samples)
    }

    /// Raffiche UART 8N1 a 10 000 baud (10 campioni per bit), separate da pause di 500 campioni.
    fn uart_wave(bursts: &[&[u8]]) -> Waveform {
        const UNIT: usize = 10;
        const PAUSA: usize = 500;
        let mut samples: Vec<f32> = Vec::new();
        for burst in bursts {
            for &byte in *burst {
                let bits = std::iter::once(false)
                    .chain((0..8).map(|b| ((byte >> b) & 1) == 1))
                    .chain(std::iter::once(true));
                for bit in bits {
                    samples.extend(std::iter::repeat_n(if bit { 3.3 } else { 0.0 }, UNIT));
                }
            }
            samples.extend(std::iter::repeat_n(3.3, PAUSA));
        }
        wave(samples)
    }

    #[test]
    fn quadra_al_cinquanta_per_cento_e_clock() {
        let c = classify(&square(100, 50, 10));
        assert_eq!(c.class, SignalClass::Clock);
        assert_eq!(c.summary, "Clock 1,00 kHz, duty 50,0 %");
        assert_eq!(c.suggested_decoders, vec!["pwm"]);
    }

    #[test]
    fn quadra_al_venti_per_cento_e_pwm() {
        assert_eq!(classify(&square(100, 20, 10)).class, SignalClass::Pwm);
    }

    #[test]
    fn sinusoide_con_pochi_periodi_e_analog() {
        // 1000 campioni a 100 kSa/s: solo 5 periodi nella FFT, la THD non è calcolabile.
        let samples: Vec<f32> = (0..1000)
            .map(|i| (2.0 * PI * 1000.0 * i as f64 / 100_000.0).sin() as f32)
            .collect();
        let c = classify(&Waveform { samples, sample_rate: 100_000.0, t0: 0.0 });
        assert_eq!(c.class, SignalClass::Analog, "{}", c.summary);
    }

    #[test]
    fn sinusoide_e_analog() {
        // 1024 campioni a 102,4 kSa/s: 1 kHz cade esattamente sul bin 10 (100 Hz per bin).
        let sr = 102_400.0;
        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * PI * 1000.0 * i as f64 / sr).sin() as f32)
            .collect();
        let c = classify(&Waveform { samples, sample_rate: sr, t0: 0.0 });
        assert_eq!(c.class, SignalClass::Analog, "{}", c.summary);
    }

    #[test]
    fn costante_e_dc() {
        assert_eq!(classify(&wave(vec![1.0; 100])).class, SignalClass::Dc);
    }

    #[test]
    fn tre_raffiche_uart_sono_serial_con_baud_stimato() {
        let bursts: [&[u8]; 3] = [&[0x55, 0x0F], &[0xA3], &[0x42, 0x7E]];
        let c = classify(&uart_wave(&bursts));
        assert_eq!(c.class, SignalClass::Serial);
        let baud = c.baud_estimate.expect("baud stimato");
        assert!((baud - 10_000.0).abs() < 1.0, "baud {baud}");
        assert_eq!(c.suggested_decoders, vec!["uart", "manchester", "onewire"]);
    }

    #[test]
    fn segment_conta_tre_pacchetti() {
        let bursts: [&[u8]; 3] = [&[0x55, 0x0F], &[0xA3], &[0x42, 0x7E]];
        let trace = digitize(&uart_wave(&bursts), None);
        assert_eq!(segment(&trace, None).len(), 3);
    }
}
