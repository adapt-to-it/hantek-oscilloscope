//! Pannello delle misure automatiche.

use eframe::egui::{Grid, Ui};
use h6022_core::Frame;
use h6022_dsp::measure::{delay_seconds, measure, phase_degrees, Measurements};

use crate::help::{self, HelpId};
use crate::state::{UiState, CH_COLORS};
use crate::timebase::{format_hz, format_time, format_volts};

const NONE: &str = "—";

/// Misure calcolate, riusate finché non arriva un frame nuovo.
#[derive(Default)]
pub struct MeasureCache {
    sequence: Option<u64>,
    measurements: [Option<Measurements>; 2],
    phase_deg: Option<f64>,
    delay_s: Option<f64>,
}

impl MeasureCache {
    pub fn update(&mut self, frame: &Frame) {
        if self.sequence == Some(frame.sequence) {
            return;
        }
        self.sequence = Some(frame.sequence);
        self.measurements = [Some(measure(&frame.ch1)), frame.ch2.as_ref().map(measure)];
        let (phase_deg, delay_s) = match &frame.ch2 {
            Some(ch2) => (phase_degrees(&frame.ch1, ch2), delay_seconds(&frame.ch1, ch2)),
            None => (None, None),
        };
        self.phase_deg = phase_deg;
        self.delay_s = delay_s;
    }

    pub fn measurements(&self, ch: usize) -> Option<&Measurements> {
        self.measurements.get(ch)?.as_ref()
    }
}

/// Tabella delle misure dei canali attivi, con sfasamento e ritardo CH1-CH2.
pub fn show(ui: &mut Ui, frame: Option<&Frame>, state: &UiState, cache: &mut MeasureCache) {
    if let Some(frame) = frame {
        cache.update(frame);
    }
    let active: Vec<usize> = (0..2).filter(|&ch| state.channels[ch].enabled).collect();
    let rows: [(&str, HelpId, fn(&Measurements) -> String); 18] = [
        ("Vpp", HelpId::Vpp, |m| format_volts(m.vpp)),
        ("Vmax", HelpId::Measurements, |m| format_volts(m.vmax)),
        ("Vmin", HelpId::Measurements, |m| format_volts(m.vmin)),
        ("Vmedia", HelpId::Measurements, |m| format_volts(m.vmean)),
        ("Vrms", HelpId::Vrms, |m| format_volts(m.vrms)),
        ("Vac rms", HelpId::Vrms, |m| format_volts(m.vac_rms)),
        ("Vtop", HelpId::Measurements, |m| format_volts(m.vtop)),
        ("Vbase", HelpId::Measurements, |m| format_volts(m.vbase)),
        ("Ampiezza", HelpId::Measurements, |m| format_volts(m.vamp)),
        ("Frequenza", HelpId::Frequency, |m| fmt_opt(m.frequency, format_hz)),
        ("Periodo", HelpId::Frequency, |m| fmt_opt(m.period, format_time)),
        ("Duty", HelpId::Duty, |m| fmt_opt(m.duty_positive, fmt_percent)),
        ("Larghezza +", HelpId::Measurements, |m| fmt_opt(m.pulse_width_positive, format_time)),
        ("Larghezza −", HelpId::Measurements, |m| fmt_opt(m.pulse_width_negative, format_time)),
        ("Salita", HelpId::RiseTime, |m| fmt_opt(m.rise_time, format_time)),
        ("Discesa", HelpId::RiseTime, |m| fmt_opt(m.fall_time, format_time)),
        ("Overshoot", HelpId::Measurements, |m| fmt_opt(m.overshoot_pct, |p| format!("{p:.1} %"))),
        ("Fronti", HelpId::Measurements, |m| format!("{} / {}", m.rising_edges, m.falling_edges)),
    ];

    Grid::new("measure_grid").striped(true).show(ui, |ui| {
        ui.label("Misura");
        for &ch in &active {
            ui.colored_label(CH_COLORS[ch], format!("CH{}", ch + 1));
        }
        ui.end_row();
        for (label, id, render) in rows.iter() {
            help::attach(ui.label(*label), *id, state);
            for &ch in &active {
                let text = cache.measurements(ch).map_or_else(|| NONE.to_string(), |m| render(m));
                ui.label(text);
            }
            ui.end_row();
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        help::attach(ui.label("Fase CH1-CH2"), HelpId::Phase, state);
        ui.label(fmt_opt(cache.phase_deg, |deg| format!("{deg:.1}°")));
    });
    ui.horizontal(|ui| {
        help::attach(ui.label("Ritardo CH1-CH2"), HelpId::Measurements, state);
        ui.label(fmt_opt(cache.delay_s, format_time));
    });
}

/// Valore formattato, o "—" se la misura non è disponibile.
fn fmt_opt<T>(value: Option<T>, format: impl Fn(T) -> String) -> String {
    value.map_or_else(|| NONE.to_string(), format)
}

/// Frazione 0..1 come percentuale.
fn fmt_percent(fraction: f64) -> String {
    format!("{:.1} %", fraction * 100.0)
}

#[cfg(test)]
mod tests {
    use h6022_core::{EngineConfig, Frame, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_dsp::Waveform;
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::MeasureCache;

    fn config() -> EngineConfig {
        EngineConfig {
            acquisition: AcquisitionConfig {
                sample_rate: SampleRate::nearest(1e6),
                mode: ChannelMode::Single,
                range: [VoltageRange::V5; 2],
                coupling: [Coupling::Dc; 2],
            },
            trigger: TriggerConfig {
                source: Channel::Ch1,
                kind: TriggerKind::Edge { slope: Slope::Rising },
                level: 0.0,
                hysteresis: 0.0,
                holdoff: 0.0,
            },
            mode: TriggerMode::Auto,
            record_length: 100,
            pretrigger: 0.25,
            probe: [1.0, 1.0],
        }
    }

    fn frame(sequence: u64, samples: Vec<f32>) -> Frame {
        Frame {
            sequence,
            ch1: Waveform { samples, sample_rate: 1e6, t0: 0.0 },
            ch2: None,
            triggered: true,
            trigger_sample: Some(0),
            config: config(),
        }
    }

    #[test]
    fn cache_does_not_recompute_for_same_sequence() {
        let square: Vec<f32> = (0..100).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let mut cache = MeasureCache::default();

        cache.update(&frame(1, square));
        let vpp = cache.measurements(0).expect("misure del primo frame").vpp;

        // Stessa sequenza con dati diversi: la cache deve restare invariata.
        cache.update(&frame(1, vec![0.5; 100]));
        assert_eq!(cache.measurements(0).expect("misure in cache").vpp, vpp);

        // Sequenza nuova: si ricalcola.
        cache.update(&frame(2, vec![0.5; 100]));
        assert_ne!(cache.measurements(0).expect("misure ricalcolate").vpp, vpp);
    }
}
