//! Canali matematici. Le operazioni tra due forme d'onda usano la lunghezza minore.

use crate::waveform::Waveform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub fn binary(a: &Waveform, b: &Waveform, op: BinaryOp) -> Waveform {
    let samples = a
        .samples
        .iter()
        .zip(b.samples.iter())
        .map(|(&x, &y)| match op {
            BinaryOp::Add => x + y,
            BinaryOp::Sub => x - y,
            BinaryOp::Mul => x * y,
            BinaryOp::Div => {
                if y == 0.0 {
                    0.0
                } else {
                    x / y
                }
            }
        })
        .collect();
    Waveform {
        samples,
        sample_rate: a.sample_rate,
        t0: a.t0,
    }
}

pub fn scale(wave: &Waveform, gain: f32, offset: f32) -> Waveform {
    Waveform {
        samples: wave.samples.iter().map(|&x| x * gain + offset).collect(),
        sample_rate: wave.sample_rate,
        t0: wave.t0,
    }
}

/// Integrale cumulativo con regola dei trapezi, in V·s.
pub fn integrate(wave: &Waveform) -> Waveform {
    let dt = if wave.sample_rate > 0.0 {
        1.0 / wave.sample_rate
    } else {
        0.0
    };
    let mut samples = Vec::with_capacity(wave.samples.len());
    let mut acc = 0.0f64;
    for (i, &x) in wave.samples.iter().enumerate() {
        if i > 0 {
            let prev = wave.samples[i - 1] as f64;
            acc += (prev + x as f64) * 0.5 * dt;
        }
        samples.push(acc as f32);
    }
    Waveform {
        samples,
        sample_rate: wave.sample_rate,
        t0: wave.t0,
    }
}

/// Derivata con differenze centrali, in V/s.
pub fn derivative(wave: &Waveform) -> Waveform {
    let x = &wave.samples;
    let n = x.len();
    // La derivata è la differenza moltiplicata per la frequenza di campionamento.
    let rate = if wave.sample_rate > 0.0 {
        wave.sample_rate
    } else {
        0.0
    };
    let samples = (0..n)
        .map(|i| {
            let d = if n < 2 {
                0.0
            } else if i == 0 {
                x[1] as f64 - x[0] as f64
            } else if i == n - 1 {
                x[n - 1] as f64 - x[n - 2] as f64
            } else {
                (x[i + 1] as f64 - x[i - 1] as f64) / 2.0
            };
            (d * rate) as f32
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

    #[test]
    fn integrate_di_un_volt_per_un_secondo_vale_un_volt_secondo() {
        // 1001 campioni a 1 kHz coprono esattamente 1 s tra primo e ultimo campione.
        let wave = Waveform {
            samples: vec![1.0f32; 1001],
            sample_rate: 1000.0,
            t0: 0.0,
        };
        let out = integrate(&wave);
        assert_eq!(out.samples[0], 0.0);
        let last = *out.samples.last().unwrap() as f64;
        assert!((last - 1.0).abs() <= 0.001, "integrale {last} V·s");
    }

    #[test]
    fn binary_usa_la_lunghezza_minore_e_divide_per_zero_da_zero() {
        let a = Waveform {
            samples: vec![1.0, 2.0, 3.0],
            sample_rate: 100.0,
            t0: -0.5,
        };
        let b = Waveform {
            samples: vec![2.0, 0.0],
            sample_rate: 50.0,
            t0: 9.0,
        };
        let out = binary(&a, &b, BinaryOp::Div);
        assert_eq!(out.samples, vec![0.5, 0.0]);
        assert_eq!(out.sample_rate, 100.0);
        assert_eq!(out.t0, -0.5);
    }

    #[test]
    fn derivative_di_rampa_e_costante_sui_campioni_attesi() {
        let wave = Waveform {
            samples: vec![0.0, 1.0, 2.0, 3.0],
            sample_rate: 10.0,
            t0: 0.0,
        };
        let out = derivative(&wave);
        assert_eq!(out.samples, vec![10.0, 10.0, 10.0, 10.0]);
    }
}
