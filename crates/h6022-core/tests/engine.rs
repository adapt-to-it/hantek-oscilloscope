use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use h6022_core::engine::{Engine, EngineHandle};
use h6022_core::sim::{SignalSpec, Shape, SimulatedSource};
use h6022_core::types::{CoreError, EngineConfig, Frame, RunState, Slope, TriggerConfig, TriggerKind, TriggerMode};
use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

const WAIT: Duration = Duration::from_secs(5);
const RECORD_LENGTH: usize = 1000;
/// Pretrigger 0.25 su 1000 campioni.
const PRE: usize = 250;

/// Quadra 0–3 V a 11 kHz su CH1 (come il banco di prova), senza rumore.
fn square_0_3v(frequency: f64) -> SignalSpec {
    SignalSpec {
        shape: Shape::Square { duty: 0.5 },
        frequency,
        amplitude: 1.5,
        offset: 1.5,
        phase_deg: 0.0,
        noise: 0.0,
    }
}

/// Continua di `volts` senza rumore.
fn dc(volts: f32) -> SignalSpec {
    SignalSpec {
        shape: Shape::Dc,
        frequency: 0.0,
        amplitude: volts,
        offset: 0.0,
        phase_deg: 0.0,
        noise: 0.0,
    }
}

fn config(mode: TriggerMode, sample_rate: SampleRate) -> EngineConfig {
    EngineConfig {
        acquisition: AcquisitionConfig {
            sample_rate,
            mode: ChannelMode::Dual,
            range: [VoltageRange::V5; 2],
            coupling: [Coupling::Dc; 2],
        },
        trigger: TriggerConfig {
            source: Channel::Ch1,
            kind: TriggerKind::Edge { slope: Slope::Rising },
            level: 1.5,
            hysteresis: 0.0,
            holdoff: 0.0,
        },
        mode,
        record_length: RECORD_LENGTH,
        pretrigger: 0.25,
        probe: [1.0, 1.0],
    }
}

fn start(signals: [SignalSpec; 2], config: EngineConfig) -> EngineHandle {
    let source = SimulatedSource::new(signals, 4096, false);
    Engine::start(Box::new(source), config).expect("avvio del motore")
}

fn next_frame(rx: &Receiver<Arc<Frame>>) -> Arc<Frame> {
    rx.recv_timeout(WAIT).expect("frame entro il timeout")
}

fn wait_until(mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !cond() {
        assert!(Instant::now() < deadline, "condizione non raggiunta in tempo");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn rising_trigger_at_1_5_v_lands_on_pretrigger_index() {
    let engine = start(
        [square_0_3v(11_000.0), square_0_3v(1_000.0)],
        config(TriggerMode::Normal, SampleRate(1)),
    );
    let rx = engine.subscribe();
    let frame = next_frame(&rx);
    engine.shutdown().unwrap();

    assert!(frame.triggered);
    assert!(frame.trigger_sample.is_some());
    let samples = &frame.ch1.samples;
    assert_eq!(samples.len(), RECORD_LENGTH);
    assert!(samples[PRE] >= 1.5, "campione di trigger {}", samples[PRE]);
    assert!(samples[PRE - 1] < 1.5, "campione precedente {}", samples[PRE - 1]);
    assert_eq!(frame.ch1.sample_rate, 1_000_000.0);
    assert!((frame.ch1.t0 + PRE as f64 / 1_000_000.0).abs() < 1e-12);
    assert!(frame.ch2.is_some());
}

#[test]
fn single_stops_after_first_frame_until_run() {
    let engine = start(
        [square_0_3v(11_000.0), square_0_3v(1_000.0)],
        config(TriggerMode::Single, SampleRate(1)),
    );
    wait_until(|| engine.state() == RunState::Stopped);
    let first = engine.latest_frame().expect("primo frame");
    assert!(first.triggered);

    let rx = engine.subscribe();
    thread::sleep(Duration::from_millis(100));
    assert!(rx.try_recv().is_err(), "nessun frame dopo il primo");
    assert_eq!(engine.state(), RunState::Stopped);

    // Con la sorgente simulata senza tempo reale il frame successivo arriva subito: lo stato
    // torna Stopped dopo ogni frame, quindi si verifica il frame e non lo stato Armed.
    engine.run();
    let again = next_frame(&rx);
    assert!(again.triggered);
    assert!(again.sequence > first.sequence);
    wait_until(|| engine.state() == RunState::Stopped);
    engine.shutdown().unwrap();
}

#[test]
fn auto_without_trigger_publishes_unsynchronized_frames() {
    let engine = start([dc(1.0), dc(1.0)], config(TriggerMode::Auto, SampleRate(1)));
    let rx = engine.subscribe();
    let frame = next_frame(&rx);
    engine.shutdown().unwrap();

    assert!(!frame.triggered);
    assert_eq!(frame.trigger_sample, None);
    assert_eq!(frame.ch1.samples.len(), RECORD_LENGTH);
    assert!((frame.ch1.t0 + PRE as f64 / 1_000_000.0).abs() < 1e-12);
}

#[test]
fn roll_publishes_last_samples_without_trigger() {
    let engine = start([square_0_3v(11_000.0), square_0_3v(1_000.0)], config(TriggerMode::Roll, SampleRate(1)));
    let rx = engine.subscribe();
    let _first = next_frame(&rx);
    let frame = next_frame(&rx);
    engine.shutdown().unwrap();

    assert!(!frame.triggered);
    assert_eq!(frame.trigger_sample, None);
    assert_eq!(frame.ch1.samples.len(), RECORD_LENGTH);
    assert!((frame.ch1.t0 + RECORD_LENGTH as f64 / 1_000_000.0).abs() < 1e-12);
}

#[test]
fn probe_multiplies_volts_before_publishing() {
    let plain = start([dc(1.0), dc(1.0)], config(TriggerMode::Auto, SampleRate(1)));
    let plain_rx = plain.subscribe();
    let plain_value = next_frame(&plain_rx).ch1.samples[0];
    plain.shutdown().unwrap();

    let mut probed_config = config(TriggerMode::Auto, SampleRate(1));
    probed_config.probe = [10.0, 1.0];
    let probed = start([dc(1.0), dc(1.0)], probed_config);
    let probed_rx = probed.subscribe();
    let probed_value = next_frame(&probed_rx).ch1.samples[0];
    probed.shutdown().unwrap();

    assert!(
        (probed_value - 10.0 * plain_value).abs() < 1e-4,
        "sonda 10: {probed_value}, sonda 1: {plain_value}"
    );
}

#[test]
fn set_config_changes_sample_rate_and_frames_follow() {
    let engine = start([dc(1.0), dc(1.0)], config(TriggerMode::Auto, SampleRate(1)));
    let rx = engine.subscribe();
    assert_eq!(next_frame(&rx).ch1.sample_rate, 1_000_000.0);

    let new_rate = SampleRate(2);
    engine.set_config(config(TriggerMode::Auto, new_rate)).unwrap();
    assert_eq!(engine.config().acquisition.sample_rate, new_rate);

    let new_hz = f64::from(new_rate.hz());
    let mut seen = false;
    for _ in 0..50 {
        let frame = next_frame(&rx);
        if frame.ch1.sample_rate == new_hz {
            assert_eq!(frame.config.acquisition.sample_rate, new_rate);
            seen = true;
            break;
        }
    }
    engine.shutdown().unwrap();
    assert!(seen, "nessun frame al nuovo sample rate");
}

#[test]
fn shutdown_stops_engine_and_later_calls_fail_with_stopped() {
    let engine = start([dc(1.0), dc(1.0)], config(TriggerMode::Auto, SampleRate(1)));
    engine.shutdown().unwrap();

    assert!(matches!(engine.shutdown(), Err(CoreError::Stopped)));
    assert!(matches!(
        engine.set_config(config(TriggerMode::Auto, SampleRate(1))),
        Err(CoreError::Stopped)
    ));
    assert_eq!(engine.state(), RunState::Stopped);
}

#[test]
fn start_rejects_record_length_below_minimum() {
    let mut bad = config(TriggerMode::Auto, SampleRate(1));
    bad.record_length = 8;
    let source = SimulatedSource::new([dc(1.0), dc(1.0)], 4096, false);
    assert!(matches!(
        Engine::start(Box::new(source), bad),
        Err(CoreError::InvalidConfig(_))
    ));
}

/// Sorgente che restituisce blocchi preparati, poi solo timeout.
struct Scripted {
    blocks: std::collections::VecDeque<h6022_core::VoltBlock>,
}

impl h6022_core::SampleSource for Scripted {
    fn configure(&mut self, _config: &AcquisitionConfig) -> h6022_core::types::CoreResult<()> {
        Ok(())
    }

    fn read(&mut self, out: &mut h6022_core::VoltBlock) -> h6022_core::types::CoreResult<()> {
        // Il primo blocco arriva dopo la sottoscrizione del test.
        thread::sleep(Duration::from_millis(50));
        match self.blocks.pop_front() {
            Some(block) => {
                *out = block;
                Ok(())
            }
            None => {
                Err(CoreError::Device(h6022_usb::Error::Timeout))
            }
        }
    }

    fn calibration(&self) -> h6022_usb::Calibration {
        h6022_usb::Calibration::default()
    }

    fn stop(&mut self) -> h6022_core::types::CoreResult<()> {
        Ok(())
    }
}

#[test]
fn triggers_and_records_stay_inside_one_segment() {
    // Segmento 1: 600 campioni a 0 V. Segmento 2 dal campione 600: 3 V, poi 0 V,
    // fronti di salita veri a 700 (troppo vicino all'interruzione per il pretrigger) e a 1400.
    let mut ch1 = vec![0.0f32; 3000];
    ch1[600..650].fill(3.0);
    ch1[700..800].fill(3.0);
    ch1[1400..1600].fill(3.0);
    let block = h6022_core::VoltBlock {
        first_sample: 0,
        ch2: vec![0.0; ch1.len()],
        ch1,
        breaks: vec![600],
    };
    let source = Scripted { blocks: [block].into() };
    let engine = Engine::start(Box::new(source), config(TriggerMode::Normal, SampleRate(1))).expect("avvio");
    let rx = engine.subscribe();
    let frame = next_frame(&rx);
    engine.shutdown().unwrap();

    // Il salto a 600 attraversa l'interruzione e il fronte a 700 non ha il pretrigger nel segmento.
    assert_eq!(frame.trigger_sample, Some(1400));
    assert!(rx.try_recv().is_err());
    assert_eq!(frame.ch1.samples.len(), RECORD_LENGTH);
    assert_eq!(frame.ch1.samples[PRE], 3.0);
}

#[test]
fn records_longer_than_segments_are_counted_as_cut() {
    // Quattro segmenti di 500 campioni, ognuno con un fronte a +300: il pretrigger (250) entra,
    // il resto del record (750) no. Il quarto segmento non si chiude e non conta.
    let mut ch1 = vec![0.0f32; 2000];
    for start in [0, 500, 1000, 1500] {
        ch1[start + 300..start + 400].fill(3.0);
    }
    let block = h6022_core::VoltBlock {
        first_sample: 0,
        ch2: vec![0.0; ch1.len()],
        ch1,
        breaks: vec![500, 1000, 1500],
    };
    let source = Scripted { blocks: [block].into() };
    let engine = Engine::start(Box::new(source), config(TriggerMode::Normal, SampleRate(1))).expect("avvio");
    let deadline = Instant::now() + WAIT;
    while engine.stats().segments < 3 {
        assert!(Instant::now() < deadline, "segmenti non contati");
        thread::sleep(Duration::from_millis(5));
    }
    let stats = engine.stats();
    engine.shutdown().unwrap();

    assert_eq!(stats.frames, 0);
    assert_eq!(stats.last_segment_samples, 500);
    assert_eq!(stats.triggers_cut, 3);
}
