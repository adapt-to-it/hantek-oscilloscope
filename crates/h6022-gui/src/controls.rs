//! Pannello dei controlli.

use eframe::egui::{Button, CollapsingHeader, ComboBox, DragValue, RichText, ScrollArea, Slider, Ui};
use h6022_core::{EngineConfig, EngineStats, Level, Polarity, RunState, Slope, TriggerKind, TriggerMode, WidthCondition};
use h6022_dsp::Window;
use h6022_usb::{Channel, ChannelMode, Coupling};

use crate::help::{self, HelpId};
use crate::state::{CursorMode, UiState, View, CH_COLORS};
use crate::timebase::{acquisition_for, format_hz, format_time, format_volts, range_for, step, time_steps, volt_steps};

/// Comando richiesto con i pulsanti.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunCommand {
    Run,
    Stop,
    Single,
}

/// Mostra i controlli. Modifica `state` e `config` sul posto; restituisce un eventuale comando.
pub fn show(
    ui: &mut Ui,
    state: &mut UiState,
    config: &mut EngineConfig,
    run_state: RunState,
    stats: &EngineStats,
) -> Option<RunCommand> {
    let before = (state.clone(), *config);
    let mut command = None;
    ScrollArea::vertical().show(ui, |ui| {
        CollapsingHeader::new("Acquisizione").default_open(true).show(ui, |ui| {
            if let Some(cmd) = acquisition_section(ui, state, config, run_state, stats) {
                command = Some(cmd);
            }
        });
        CollapsingHeader::new("Orizzontale").default_open(true).show(ui, |ui| {
            horizontal_section(ui, state, config);
        });
        for ch in 0..2 {
            CollapsingHeader::new(RichText::new(format!("CH{}", ch + 1)).color(CH_COLORS[ch]))
                .default_open(true)
                .show(ui, |ui| channel_section(ui, state, config, ch));
        }
        CollapsingHeader::new("Trigger").default_open(true).show(ui, |ui| {
            trigger_section(ui, state, config);
        });
        CollapsingHeader::new("Display").default_open(true).show(ui, |ui| {
            display_section(ui, state);
        });
    });
    if *state != before.0 || *config != before.1 {
        sync_config(state, config);
    }
    command
}

/// Allinea `config` alle scelte di `state`: frequenza e record da `time_per_div`,
/// modalità a uno o due canali, portate da `volts_per_div` e offset.
pub fn sync_config(state: &UiState, config: &mut EngineConfig) {
    let mode = if state.channels[1].enabled { ChannelMode::Dual } else { ChannelMode::Single };
    let (rate, record) = acquisition_for(state.time_per_div, mode);
    config.acquisition.sample_rate = rate;
    config.acquisition.mode = mode;
    config.record_length = record;
    for ch in 0..2 {
        let view = state.channels[ch];
        config.acquisition.range[ch] = range_for(view.volts_per_div, view.offset_div, config.probe[ch]);
    }
}

fn acquisition_section(
    ui: &mut Ui,
    state: &UiState,
    config: &mut EngineConfig,
    run_state: RunState,
    stats: &EngineStats,
) -> Option<RunCommand> {
    let mut command = None;
    ui.horizontal(|ui| {
        let run = ui.add(Button::new("Run").selected(run_state == RunState::Running));
        if help::attach(run, HelpId::Run, state).clicked() {
            command = Some(RunCommand::Run);
        }
        let stop = ui.add(Button::new("Stop"));
        if help::attach(stop, HelpId::Stop, state).clicked() {
            command = Some(RunCommand::Stop);
        }
        let single = ui.add(Button::new("Single").selected(run_state == RunState::Armed));
        if help::attach(single, HelpId::Single, state).clicked() {
            command = Some(RunCommand::Single);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Trigger");
        let resp = ComboBox::from_id_salt("trigger_mode")
            .selected_text(trigger_mode_name(config.mode))
            .show_ui(ui, |ui| {
                for mode in [TriggerMode::Auto, TriggerMode::Normal, TriggerMode::Single, TriggerMode::Roll] {
                    ui.selectable_value(&mut config.mode, mode, trigger_mode_name(mode));
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerMode, state);
    });
    ui.label(format!(
        "Frequenza {}, record {} campioni",
        format_hz(f64::from(config.acquisition.sample_rate.hz())),
        config.record_length
    ));
    ui.label(format!("Misurata {}", format_hz(stats.sample_rate_measured)));
    command
}

fn horizontal_section(ui: &mut Ui, state: &mut UiState, config: &mut EngineConfig) {
    let times = time_steps();
    ui.horizontal(|ui| {
        ui.label("Tempo/div");
        let minus = ui.button("−");
        if help::attach(minus, HelpId::TimePerDiv, state).clicked() {
            state.time_per_div = step(times, state.time_per_div, -1);
        }
        let plus = ui.button("+");
        if help::attach(plus, HelpId::TimePerDiv, state).clicked() {
            state.time_per_div = step(times, state.time_per_div, 1);
        }
        let resp = ComboBox::from_id_salt("time_per_div")
            .selected_text(format_time(state.time_per_div))
            .show_ui(ui, |ui| {
                for &t in times {
                    ui.selectable_value(&mut state.time_per_div, t, format_time(t));
                }
            })
            .response;
        help::attach(resp, HelpId::TimePerDiv, state);
    });
    ui.horizontal(|ui| {
        ui.label("Offset");
        let mut divisions = state.horizontal_offset / state.time_per_div;
        let drag = ui.add(DragValue::new(&mut divisions).speed(0.05).suffix(" div"));
        if help::attach(drag, HelpId::HorizontalOffset, state).changed() {
            state.horizontal_offset = divisions * state.time_per_div;
        }
        let centre = ui.button("Centra");
        if help::attach(centre, HelpId::HorizontalOffset, state).clicked() {
            state.horizontal_offset = 0.0;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Pretrigger");
        let mut percent = config.pretrigger * 100.0;
        let slider = ui.add(Slider::new(&mut percent, 0.0..=100.0).suffix(" %"));
        if help::attach(slider, HelpId::Pretrigger, state).changed() {
            config.pretrigger = percent / 100.0;
        }
    });
}

fn channel_section(ui: &mut Ui, state: &mut UiState, config: &mut EngineConfig, ch: usize) {
    let probe = config.probe[ch];
    // La lista delle portate è in volt all'ingresso della sonda: i passi dell'oscilloscopio per la sonda.
    let steps: Vec<f32> = volt_steps().iter().map(|v| v * probe).collect();

    let enabled = ui.checkbox(&mut state.channels[ch].enabled, "Attivo");
    help::attach(enabled, HelpId::ChannelEnable, state);

    ui.horizontal(|ui| {
        ui.label("V/div");
        let minus = ui.button("−");
        if help::attach(minus, HelpId::VoltsPerDiv, state).clicked() {
            state.channels[ch].volts_per_div = step(&steps, state.channels[ch].volts_per_div, -1);
        }
        let plus = ui.button("+");
        if help::attach(plus, HelpId::VoltsPerDiv, state).clicked() {
            state.channels[ch].volts_per_div = step(&steps, state.channels[ch].volts_per_div, 1);
        }
        let resp = ComboBox::from_id_salt("volts_per_div")
            .selected_text(format_volts(state.channels[ch].volts_per_div))
            .show_ui(ui, |ui| {
                for &v in &steps {
                    ui.selectable_value(&mut state.channels[ch].volts_per_div, v, format_volts(v));
                }
            })
            .response;
        help::attach(resp, HelpId::VoltsPerDiv, state);
    });

    ui.horizontal(|ui| {
        ui.label("Offset");
        let mut divisions = state.channels[ch].offset_div;
        let drag = ui.add(DragValue::new(&mut divisions).speed(0.05).suffix(" div"));
        if help::attach(drag, HelpId::VerticalOffset, state).changed() {
            state.channels[ch].offset_div = divisions;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Sonda");
        let resp = ComboBox::from_id_salt("probe")
            .selected_text(probe_name(config.probe[ch]))
            .show_ui(ui, |ui| {
                for p in [1.0_f32, 10.0] {
                    ui.selectable_value(&mut config.probe[ch], p, probe_name(p));
                }
            })
            .response;
        help::attach(resp, HelpId::Probe, state);
    });

    ui.horizontal(|ui| {
        ui.label("Accoppiamento");
        let resp = ComboBox::from_id_salt("coupling")
            .selected_text(coupling_name(config.acquisition.coupling[ch]))
            .show_ui(ui, |ui| {
                for c in [Coupling::Ac, Coupling::Dc] {
                    ui.selectable_value(&mut config.acquisition.coupling[ch], c, coupling_name(c));
                }
            })
            .response;
        help::attach(resp, HelpId::Coupling, state);
    });
}

fn trigger_section(ui: &mut Ui, state: &mut UiState, config: &mut EngineConfig) {
    ui.horizontal(|ui| {
        ui.label("Sorgente");
        let resp = ComboBox::from_id_salt("trigger_source")
            .selected_text(channel_name(config.trigger.source))
            .show_ui(ui, |ui| {
                for c in [Channel::Ch1, Channel::Ch2] {
                    ui.selectable_value(&mut config.trigger.source, c, channel_name(c));
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerSource, state);
    });

    let current = kind_index(&config.trigger.kind);
    let mut chosen = current;
    ui.horizontal(|ui| {
        ui.label("Tipo");
        let resp = ComboBox::from_id_salt("trigger_kind")
            .selected_text(KIND_NAMES[current])
            .show_ui(ui, |ui| {
                for (i, name) in KIND_NAMES.iter().enumerate() {
                    if ui.selectable_label(i == current, *name).clicked() {
                        chosen = i;
                    }
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerKind, state);
    });
    if chosen != current {
        config.trigger.kind = default_kind(chosen);
    }

    match &mut config.trigger.kind {
        TriggerKind::Edge { slope } => {
            slope_combo(ui, state, "edge_slope", "Fronte", slope, HelpId::TriggerSlope);
        }
        TriggerKind::PulseWidth { polarity, condition } => {
            polarity_combo(ui, state, "pulse_polarity", polarity);
            pulse_condition(ui, state, condition);
        }
        TriggerKind::Runt { polarity, low, high } => {
            polarity_combo(ui, state, "runt_polarity", polarity);
            volts_drag(ui, state, "Basso", low, HelpId::TriggerKind);
            volts_drag(ui, state, "Alto", high, HelpId::TriggerKind);
        }
        TriggerKind::Window { low, high, enter } => {
            volts_drag(ui, state, "Basso", low, HelpId::TriggerKind);
            volts_drag(ui, state, "Alto", high, HelpId::TriggerKind);
            ui.horizontal(|ui| {
                let resp = ui.checkbox(enter, "Entra nella fascia");
                help::attach(resp, HelpId::TriggerKind, state);
            });
        }
        TriggerKind::Timeout { slope, seconds } => {
            slope_combo(ui, state, "timeout_slope", "Fronte", slope, HelpId::TriggerSlope);
            time_drag(ui, state, "Timeout", seconds, HelpId::TriggerKind);
        }
        TriggerKind::Pattern { ch1, ch2 } => {
            level_combo(ui, state, "pattern_ch1", "CH1", ch1);
            level_combo(ui, state, "pattern_ch2", "CH2", ch2);
        }
    }

    volts_drag(ui, state, "Livello", &mut config.trigger.level, HelpId::TriggerLevel);
    volts_drag(ui, state, "Isteresi", &mut config.trigger.hysteresis, HelpId::Hysteresis);
    let mut holdoff = config.trigger.holdoff;
    time_drag(ui, state, "Holdoff", &mut holdoff, HelpId::Holdoff);
    config.trigger.holdoff = holdoff;
}

fn display_section(ui: &mut Ui, state: &mut UiState) {
    ui.horizontal(|ui| {
        ui.label("Vista");
        let resp = ComboBox::from_id_salt("view")
            .selected_text(view_name(state.view))
            .show_ui(ui, |ui| {
                for v in [View::Time, View::Spectrum, View::Xy] {
                    ui.selectable_value(&mut state.view, v, view_name(v));
                }
            })
            .response;
        help::attach(resp, HelpId::View, state);
    });
    ui.horizontal(|ui| {
        ui.label("Finestra FFT");
        let resp = ComboBox::from_id_salt("fft_window")
            .selected_text(window_name(state.window))
            .show_ui(ui, |ui| {
                for w in Window::all() {
                    ui.selectable_value(&mut state.window, w, window_name(w));
                }
            })
            .response;
        help::attach(resp, HelpId::FftWindow, state);
    });
    let log = ui.checkbox(&mut state.spectrum_log_freq, "Asse delle frequenze logaritmico");
    help::attach(log, HelpId::FftWindow, state);
    let persistence = ui.checkbox(&mut state.persistence, "Persistenza");
    help::attach(persistence, HelpId::Persistence, state);
    ui.horizontal(|ui| {
        ui.label("Cursori");
        let resp = ComboBox::from_id_salt("cursor_mode")
            .selected_text(cursor_name(state.cursors.mode))
            .show_ui(ui, |ui| {
                for m in [CursorMode::Off, CursorMode::Time, CursorMode::Voltage, CursorMode::Both] {
                    ui.selectable_value(&mut state.cursors.mode, m, cursor_name(m));
                }
            })
            .response;
        help::attach(resp, HelpId::Cursors, state);
        let resp = ComboBox::from_id_salt("cursor_channel")
            .selected_text(channel_name(state.cursors.channel))
            .show_ui(ui, |ui| {
                for c in [Channel::Ch1, Channel::Ch2] {
                    ui.selectable_value(&mut state.cursors.channel, c, channel_name(c));
                }
            })
            .response;
        help::attach(resp, HelpId::Cursors, state);
    });
    ui.separator();
    ui.checkbox(&mut state.help, "Spiegazioni");
    ui.weak("Suggerimento: tasto i");
}

/// Nomi dei tipi di trigger, nell'ordine di `kind_index`.
const KIND_NAMES: [&str; 6] = ["Edge", "PulseWidth", "Runt", "Window", "Timeout", "Pattern"];
/// Nomi delle condizioni sulla durata, nell'ordine di `width_parts`.
const WIDTH_NAMES: [&str; 4] = ["Minore di", "Maggiore di", "Dentro", "Fuori"];

fn kind_index(kind: &TriggerKind) -> usize {
    match kind {
        TriggerKind::Edge { .. } => 0,
        TriggerKind::PulseWidth { .. } => 1,
        TriggerKind::Runt { .. } => 2,
        TriggerKind::Window { .. } => 3,
        TriggerKind::Timeout { .. } => 4,
        TriggerKind::Pattern { .. } => 5,
    }
}

/// Tipo di trigger con parametri di partenza ragionevoli per la scelta dell'utente.
fn default_kind(index: usize) -> TriggerKind {
    match index {
        0 => TriggerKind::Edge { slope: Slope::Rising },
        1 => TriggerKind::PulseWidth {
            polarity: Polarity::Positive,
            condition: WidthCondition::GreaterThan(10e-6),
        },
        2 => TriggerKind::Runt { polarity: Polarity::Positive, low: 0.5, high: 1.0 },
        3 => TriggerKind::Window { low: -0.5, high: 0.5, enter: true },
        4 => TriggerKind::Timeout { slope: Slope::Rising, seconds: 1e-3 },
        _ => TriggerKind::Pattern { ch1: Level::High, ch2: Level::DontCare },
    }
}

/// Indice della condizione, e i due tempi (il secondo usato solo da Within e Outside).
fn width_parts(condition: WidthCondition) -> (usize, f64, f64) {
    match condition {
        WidthCondition::LessThan(a) => (0, a, a),
        WidthCondition::GreaterThan(a) => (1, a, a),
        WidthCondition::Within(a, b) => (2, a, b),
        WidthCondition::Outside(a, b) => (3, a, b),
    }
}

fn width_from(index: usize, a: f64, b: f64) -> WidthCondition {
    match index {
        0 => WidthCondition::LessThan(a),
        1 => WidthCondition::GreaterThan(a),
        2 => WidthCondition::Within(a, b),
        _ => WidthCondition::Outside(a, b),
    }
}

fn pulse_condition(ui: &mut Ui, state: &UiState, condition: &mut WidthCondition) {
    let (mut index, mut a, mut b) = width_parts(*condition);
    ui.horizontal(|ui| {
        ui.label("Durata");
        let resp = ComboBox::from_id_salt("pulse_condition")
            .selected_text(WIDTH_NAMES[index])
            .show_ui(ui, |ui| {
                for (i, name) in WIDTH_NAMES.iter().enumerate() {
                    ui.selectable_value(&mut index, i, *name);
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerKind, state);
    });
    time_drag(ui, state, "Valore", &mut a, HelpId::TriggerKind);
    if index >= 2 {
        time_drag(ui, state, "Fino a", &mut b, HelpId::TriggerKind);
    }
    *condition = width_from(index, a, b);
}

fn trigger_mode_name(mode: TriggerMode) -> &'static str {
    match mode {
        TriggerMode::Auto => "Auto",
        TriggerMode::Normal => "Normal",
        TriggerMode::Single => "Single",
        TriggerMode::Roll => "Roll",
    }
}

fn view_name(view: View) -> &'static str {
    match view {
        View::Time => "Tempo",
        View::Spectrum => "Spettro",
        View::Xy => "XY",
    }
}

fn window_name(window: Window) -> &'static str {
    match window {
        Window::Rectangular => "Rettangolare",
        Window::Hann => "Hann",
        Window::Hamming => "Hamming",
        Window::BlackmanHarris => "Blackman-Harris",
        Window::FlatTop => "Flat-top",
    }
}

fn cursor_name(mode: CursorMode) -> &'static str {
    match mode {
        CursorMode::Off => "Off",
        CursorMode::Time => "Tempo",
        CursorMode::Voltage => "Tensione",
        CursorMode::Both => "Entrambi",
    }
}

fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Ch1 => "CH1",
        Channel::Ch2 => "CH2",
    }
}

fn coupling_name(coupling: Coupling) -> &'static str {
    match coupling {
        Coupling::Ac => "AC",
        Coupling::Dc => "DC",
    }
}

fn probe_name(probe: f32) -> String {
    format!("x{probe:.0}")
}

fn polarity_name(polarity: Polarity) -> &'static str {
    match polarity {
        Polarity::Positive => "Positivo",
        Polarity::Negative => "Negativo",
    }
}

fn slope_name(slope: Slope) -> &'static str {
    match slope {
        Slope::Rising => "Salita",
        Slope::Falling => "Discesa",
        Slope::Either => "Entrambi",
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::High => "Alto",
        Level::Low => "Basso",
        Level::DontCare => "Indifferente",
    }
}

fn polarity_combo(ui: &mut Ui, state: &UiState, salt: &'static str, polarity: &mut Polarity) {
    ui.horizontal(|ui| {
        ui.label("Polarità");
        let resp = ComboBox::from_id_salt(salt)
            .selected_text(polarity_name(*polarity))
            .show_ui(ui, |ui| {
                for p in [Polarity::Positive, Polarity::Negative] {
                    ui.selectable_value(polarity, p, polarity_name(p));
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerKind, state);
    });
}

fn slope_combo(ui: &mut Ui, state: &UiState, salt: &'static str, label: &str, slope: &mut Slope, id: HelpId) {
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ComboBox::from_id_salt(salt)
            .selected_text(slope_name(*slope))
            .show_ui(ui, |ui| {
                for s in [Slope::Rising, Slope::Falling, Slope::Either] {
                    ui.selectable_value(slope, s, slope_name(s));
                }
            })
            .response;
        help::attach(resp, id, state);
    });
}

fn level_combo(ui: &mut Ui, state: &UiState, salt: &'static str, label: &str, level: &mut Level) {
    ui.horizontal(|ui| {
        ui.label(label);
        let resp = ComboBox::from_id_salt(salt)
            .selected_text(level_name(*level))
            .show_ui(ui, |ui| {
                for l in [Level::High, Level::Low, Level::DontCare] {
                    ui.selectable_value(level, l, level_name(l));
                }
            })
            .response;
        help::attach(resp, HelpId::TriggerKind, state);
    });
}

fn volts_drag(ui: &mut Ui, state: &UiState, label: &str, value: &mut f32, id: HelpId) {
    ui.horizontal(|ui| {
        ui.label(label);
        let drag = ui.add(DragValue::new(value).speed(0.01).custom_formatter(|v, _| format_volts(v as f32)));
        help::attach(drag, id, state);
    });
}

fn time_drag(ui: &mut Ui, state: &UiState, label: &str, seconds: &mut f64, id: HelpId) {
    let speed = seconds.abs().max(1e-7) * 0.05;
    ui.horizontal(|ui| {
        ui.label(label);
        let drag = ui.add(DragValue::new(seconds).speed(speed).custom_formatter(|v, _| format_time(v)));
        help::attach(drag, id, state);
    });
}

#[cfg(test)]
mod tests {
    use h6022_core::{EngineConfig, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::sync_config;
    use crate::state::UiState;

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
                level: 0.5,
                hysteresis: 0.1,
                holdoff: 0.0,
            },
            mode: TriggerMode::Auto,
            record_length: 1000,
            pretrigger: 0.25,
            probe: [1.0, 1.0],
        }
    }

    #[test]
    fn sync_uses_single_channel_mode_when_ch2_is_off() {
        let mut state = UiState::default();
        state.channels[1].enabled = false;
        let mut cfg = config();
        sync_config(&state, &mut cfg);
        assert_eq!(cfg.acquisition.mode, ChannelMode::Single);
    }

    #[test]
    fn sync_uses_dual_channel_mode_when_ch2_is_on() {
        let state = UiState::default();
        let mut cfg = config();
        sync_config(&state, &mut cfg);
        assert_eq!(cfg.acquisition.mode, ChannelMode::Dual);
    }

    #[test]
    fn sync_leaves_trigger_untouched() {
        let state = UiState::default();
        let mut cfg = config();
        let trigger = cfg.trigger;
        sync_config(&state, &mut cfg);
        assert_eq!(cfg.trigger, trigger);
    }
}
