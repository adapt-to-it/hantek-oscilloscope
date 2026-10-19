//! Oscilloscopio simulato: genera segnali sintetici quantizzati come l'ADC a 8 bit reale.

use std::f64::consts::TAU;
use std::thread;
use std::time::Duration;

use h6022_usb::{AcquisitionConfig, Calibration, Channel, ChannelMode};
#[cfg(test)]
use h6022_usb::VoltageRange;

use crate::source::{SampleSource, VoltBlock};
use crate::types::{CoreError, CoreResult};

/// Campioni per canale a ogni `read` del banco di prova.
const BENCH_BLOCK_LEN: usize = 4096;
/// Seme fisso del generatore xorshift64 (diverso da zero).
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Sine,
    Square { duty: f64 },
    Triangle,
    Sawtooth,
    Dc,
    Noise,
}

/// Segnale di un canale: `offset + amplitude * forma(t)`, forma in -1..1 (Dc vale 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalSpec {
    pub shape: Shape,
    pub frequency: f64,
    /// Ampiezza di picco in volt.
    pub amplitude: f32,
    pub offset: f32,
    /// Fase iniziale in gradi.
    pub phase_deg: f64,
    /// Deviazione standard del rumore gaussiano in volt.
    pub noise: f32,
}

pub struct SimulatedSource {
    signals: [SignalSpec; 2],
    config: Option<AcquisitionConfig>,
    block_len: usize,
    realtime: bool,
    /// Indice del prossimo campione dall'ultima `configure`: il tempo è `sample / hz`.
    sample: u64,
    /// Stato del generatore xorshift64 per il rumore.
    rng: u64,
}

impl SimulatedSource {
    /// `block_len` campioni per canale a ogni `read`. Con `realtime` la lettura
    /// attende il tempo che il dispositivo reale impiegherebbe.
    pub fn new(signals: [SignalSpec; 2], block_len: usize, realtime: bool) -> SimulatedSource {
        SimulatedSource {
            signals,
            config: None,
            block_len,
            realtime,
            sample: 0,
            rng: SEED,
        }
    }

    /// I segnali collegati al dispositivo di prova: CH1 quadra 0–3 V 11 kHz, CH2 quadra 0–2 V 1 kHz.
    pub fn bench_default() -> SimulatedSource {
        SimulatedSource::new(bench_signals(), BENCH_BLOCK_LEN, true)
    }

    pub fn set_signal(&mut self, channel: Channel, spec: SignalSpec) {
        self.signals[channel_index(channel)] = spec;
    }

    /// Genera `block_len` campioni del canale a partire da `self.sample`, quantizzati.
    fn fill(&mut self, channel: Channel, config: AcquisitionConfig, out: &mut Vec<f32>) {
        let range = config.range[channel_index(channel)];
        let spec = self.signals[channel_index(channel)];
        let hz = f64::from(config.sample_rate.hz());
        let code = f64::from(range.code());
        let calibration = Calibration::default();

        out.clear();
        out.reserve(self.block_len);
        for i in 0..self.block_len {
            let t = (self.sample + i as u64) as f64 / hz;
            let noise = f64::from(spec.noise) * self.gaussian();
            let v = f64::from(spec.offset)
                + f64::from(spec.amplitude) * shape_value(spec.shape, spec.frequency, spec.phase_deg, t)
                + noise;
            // Come l'ADC reale: conteggi a 8 bit con zero a 128, poi di nuovo in volt.
            let raw = (v * code * 128.0 / 5.12 + 128.0).round().clamp(0.0, 255.0) as u8;
            out.push(calibration.to_volts(channel, range, raw));
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    /// Uniforme in (0, 1]: il logaritmo non riceve mai zero.
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 1.0) / (1u64 << 53) as f64
    }

    /// Campione gaussiano standard con Box-Muller.
    fn gaussian(&mut self) -> f64 {
        let u1 = self.uniform();
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

/// Forma d'onda in -1..1 al tempo `t` secondi. Il rumore è aggiunto a parte.
fn shape_value(shape: Shape, frequency: f64, phase_deg: f64, t: f64) -> f64 {
    let p = (t * frequency + phase_deg / 360.0).rem_euclid(1.0);
    match shape {
        Shape::Sine => (TAU * p).sin(),
        Shape::Square { duty } => {
            if p < duty {
                1.0
            } else {
                -1.0
            }
        }
        Shape::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Shape::Sawtooth => 2.0 * p - 1.0,
        Shape::Dc => 1.0,
        Shape::Noise => 0.0,
    }
}

fn bench_signals() -> [SignalSpec; 2] {
    [
        SignalSpec {
            shape: Shape::Square { duty: 0.5 },
            frequency: 11_000.0,
            amplitude: 1.5,
            offset: 1.5,
            phase_deg: 0.0,
            noise: 0.005,
        },
        SignalSpec {
            shape: Shape::Square { duty: 0.5 },
            frequency: 1_000.0,
            amplitude: 1.0,
            offset: 1.0,
            phase_deg: 0.0,
            noise: 0.005,
        },
    ]
}

fn channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Ch1 => 0,
        Channel::Ch2 => 1,
    }
}

impl SampleSource for SimulatedSource {
    fn configure(&mut self, config: &AcquisitionConfig) -> CoreResult<()> {
        if config.sample_rate.hz() == 0 {
            return Err(CoreError::InvalidConfig(format!(
                "frequenza di campionamento sconosciuta: id {}",
                config.sample_rate.0
            )));
        }
        self.config = Some(*config);
        self.sample = 0;
        Ok(())
    }

    fn read(&mut self, out: &mut VoltBlock) -> CoreResult<()> {
        let config = self
            .config
            .ok_or_else(|| CoreError::InvalidConfig("read prima di configure".to_string()))?;
        if self.realtime {
            let hz = f64::from(config.sample_rate.hz());
            thread::sleep(Duration::from_secs_f64(self.block_len as f64 / hz));
        }

        out.first_sample = self.sample;
        out.breaks.clear();
        self.fill(Channel::Ch1, config, &mut out.ch1);
        match config.mode {
            ChannelMode::Single => out.ch2.clear(),
            ChannelMode::Dual => self.fill(Channel::Ch2, config, &mut out.ch2),
        }
        self.sample += self.block_len as u64;
        Ok(())
    }

    fn calibration(&self) -> Calibration {
        Calibration::default()
    }

    fn stop(&mut self) -> CoreResult<()> {
        self.config = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use h6022_usb::{Coupling, SampleRate};

    #[test]
    fn bench_ch1_mean_and_range_at_1_mhz_v5() {
        let mut src = SimulatedSource::new(bench_signals(), 100_000, false);
        src.configure(&AcquisitionConfig {
            sample_rate: SampleRate(1),
            mode: ChannelMode::Dual,
            range: [VoltageRange::V5; 2],
            coupling: [Coupling::Dc; 2],
        })
        .unwrap();

        let mut out = VoltBlock::default();
        src.read(&mut out).unwrap();

        assert_eq!(out.ch1.len(), 100_000);
        assert_eq!(out.ch2.len(), 100_000);
        let mean = out.ch1.iter().map(|&v| f64::from(v)).sum::<f64>() / out.ch1.len() as f64;
        assert!((mean - 1.5).abs() < 0.05, "media CH1 {mean}");
        assert!(out.ch1.iter().all(|&v| (-0.1..=3.1).contains(&v)));
    }

    #[test]
    fn read_before_configure_is_invalid_config() {
        let mut src = SimulatedSource::bench_default();
        let mut out = VoltBlock::default();
        assert!(matches!(src.read(&mut out), Err(CoreError::InvalidConfig(_))));
    }

    #[test]
    fn single_mode_leaves_ch2_empty() {
        let mut src = SimulatedSource::new(bench_signals(), 64, false);
        src.configure(&AcquisitionConfig {
            sample_rate: SampleRate(1),
            mode: ChannelMode::Single,
            range: [VoltageRange::V5; 2],
            coupling: [Coupling::Dc; 2],
        })
        .unwrap();
        let mut out = VoltBlock::default();
        src.read(&mut out).unwrap();
        assert_eq!(out.ch1.len(), 64);
        assert!(out.ch2.is_empty());
    }

    #[test]
    fn indices_continue_between_reads_and_restart_on_configure() {
        let mut src = SimulatedSource::new(bench_signals(), 100, false);
        let config = AcquisitionConfig {
            sample_rate: SampleRate(1),
            mode: ChannelMode::Dual,
            range: [VoltageRange::V5; 2],
            coupling: [Coupling::Dc; 2],
        };
        src.configure(&config).unwrap();
        let mut out = VoltBlock::default();
        src.read(&mut out).unwrap();
        assert_eq!(out.first_sample, 0);
        src.read(&mut out).unwrap();
        assert_eq!(out.first_sample, 100);
        src.configure(&config).unwrap();
        src.read(&mut out).unwrap();
        assert_eq!(out.first_sample, 0);
    }
}
