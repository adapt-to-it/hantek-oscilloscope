//! Analisi PWM per periodo e servocomandi RC.

use crate::{Annotation, LogicTrace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PwmCycle {
    pub start_s: f64,
    pub period_s: f64,
    pub high_s: f64,
    /// 0..1.
    pub duty: f64,
}

/// Un elemento per ogni periodo completo (da fronte di salita a fronte di salita).
pub fn cycles(trace: &LogicTrace) -> Vec<PwmCycle> {
    // Segmenti alti che iniziano con un fronte di salita. Il segmento alto iniziale della
    // traccia, se la traccia parte alta, non ha fronte di salita.
    let highs: Vec<(usize, usize)> = trace
        .runs()
        .iter()
        .enumerate()
        .filter(|(i, r)| r.2 && !(*i == 0 && trace.initial))
        .map(|(_, r)| (r.0, r.1))
        .collect();
    highs
        .windows(2)
        .map(|w| {
            let (r0, e0) = w[0];
            let r1 = w[1].0;
            let period_s = (r1 - r0) as f64 / trace.sample_rate;
            let high_s = (e0 - r0) as f64 / trace.sample_rate;
            PwmCycle {
                start_s: trace.time(r0),
                period_s,
                high_s,
                duty: high_s / period_s,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PwmStats {
    pub cycles: usize,
    pub frequency_hz: f64,
    pub duty_mean: f64,
    pub duty_min: f64,
    pub duty_max: f64,
    /// Deviazione standard del periodo, in secondi.
    pub period_jitter_s: f64,
}

/// `None` senza cicli completi.
pub fn stats(trace: &LogicTrace) -> Option<PwmStats> {
    let c = cycles(trace);
    if c.is_empty() {
        return None;
    }
    let n = c.len() as f64;
    let mean_period = c.iter().map(|x| x.period_s).sum::<f64>() / n;
    let variance = c.iter().map(|x| (x.period_s - mean_period).powi(2)).sum::<f64>() / n;
    Some(PwmStats {
        cycles: c.len(),
        frequency_hz: 1.0 / mean_period,
        duty_mean: c.iter().map(|x| x.duty).sum::<f64>() / n,
        duty_min: c.iter().map(|x| x.duty).fold(f64::INFINITY, f64::min),
        duty_max: c.iter().map(|x| x.duty).fold(f64::NEG_INFINITY, f64::max),
        period_jitter_s: variance.sqrt(),
    })
}

/// Servo RC: periodo 2,5..30 ms e impulso 0,5..2,5 ms. Annotazioni "data" per impulso con testo
/// "1,50 ms (90°)" (angolo = (impulso - 1 ms) * 180°, limitato a 0..180), value = microsecondi;
/// "error" per impulsi fuori intervallo.
pub fn decode_servo(trace: &LogicTrace) -> Vec<Annotation> {
    cycles(trace)
        .into_iter()
        .filter(|c| (0.0025..=0.030).contains(&c.period_s))
        .map(|c| {
            let ms = c.high_s * 1000.0;
            let end = c.start_s + c.high_s;
            if (0.0005..=0.0025).contains(&c.high_s) {
                let angle = ((ms - 1.0) * 180.0).clamp(0.0, 180.0);
                let text = format!("{ms:.2} ms ({angle:.0}°)").replace('.', ",");
                Annotation::new(c.start_s, end, "data", text).with_value((c.high_s * 1e6).round() as u64)
            } else {
                let text = format!("{ms:.2} ms fuori intervallo").replace('.', ",");
                Annotation::new(c.start_s, end, "error", text).as_error()
            }
        })
        .collect()
}

/// Annotazioni "data" per ciclo con testo "duty 50,0 %", value = duty in permille.
pub fn annotate(trace: &LogicTrace) -> Vec<Annotation> {
    cycles(trace)
        .into_iter()
        .map(|c| {
            let text = format!("duty {:.1} %", c.duty * 100.0).replace('.', ",");
            Annotation::new(c.start_s, c.start_s + c.period_s, "data", text)
                .with_value((c.duty * 1000.0).round() as u64)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traccia a 1 MHz da segmenti `(livello, campioni)`.
    fn trace(segs: &[(bool, usize)]) -> LogicTrace {
        let initial = segs[0].0;
        let mut transitions = Vec::new();
        let mut level = initial;
        let mut pos = 0;
        for &(lvl, len) in segs {
            if lvl != level {
                transitions.push(pos);
                level = lvl;
            }
            pos += len;
        }
        LogicTrace::from_transitions(1_000_000.0, pos, initial, transitions)
    }

    #[test]
    fn pwm_1khz_25_percent() {
        let segs: Vec<(bool, usize)> = (0..10).flat_map(|_| [(false, 750), (true, 250)]).collect();
        let t = trace(&segs);
        let c = cycles(&t);
        assert_eq!(c.len(), 9);
        assert!((c[0].duty - 0.25).abs() < 1e-9);
        let s = stats(&t).unwrap();
        assert!((s.duty_mean - 0.25).abs() < 0.01);
        assert!((s.frequency_hz - 1000.0).abs() < 1.0);
        let a = annotate(&t);
        assert_eq!(a.len(), 9);
        assert_eq!(a[0].text, "duty 25,0 %");
    }

    #[test]
    fn servo_1_5_ms_is_90_degrees() {
        let segs: Vec<(bool, usize)> = (0..5).flat_map(|_| [(false, 18500), (true, 1500)]).collect();
        let a = decode_servo(&trace(&segs));
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].text, "1,50 ms (90°)");
        assert_eq!(a[0].value, Some(1500));
        assert!(!a[0].error);
    }

    #[test]
    fn servo_3_ms_is_error() {
        let segs: Vec<(bool, usize)> = (0..5).flat_map(|_| [(false, 17000), (true, 3000)]).collect();
        let a = decode_servo(&trace(&segs));
        assert_eq!(a.len(), 4);
        assert!(a.iter().all(|x| x.error && x.kind == "error"));
    }
}
