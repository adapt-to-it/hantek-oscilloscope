//! Pannello dei decoder: scelta del protocollo tra i controlli e annotazioni
//! in una striscia sotto il display.

use eframe::egui::{pos2, vec2, Align2, Color32, CollapsingHeader, ComboBox, DragValue, FontId, Painter, Rect, Sense, Ui};
use h6022_core::Frame;
use h6022_decode::can::CanConfig;
use h6022_decode::protocol;
use h6022_decode::uart::{Parity, UartConfig};
use h6022_decode::{Annotation, Protocol};

use crate::help::{self, HelpId};
use crate::scope_view::time_to_x;
use crate::state::UiState;
use crate::timebase::format_volts;

/// Altezza della striscia delle annotazioni, in punti.
pub const STRIP_HEIGHT: f32 = 22.0;
/// Altezza della riga di riepilogo sotto la striscia.
const SUMMARY_HEIGHT: f32 = 18.0;
/// Spazio da lasciare libero sotto il display quando il decoder è attivo.
pub const RESERVED_HEIGHT: f32 = STRIP_HEIGHT + SUMMARY_HEIGHT;
/// Larghezza minima di un'annotazione sullo schermo.
const MIN_WIDTH: f32 = 2.0;
/// Larghezza approssimata di un carattere monospazio a 11 punti, con margine.
const CHAR_WIDTH: f32 = 6.6;
const TEXT: Color32 = Color32::from_gray(190);

/// Un protocollo per ciascuna voce della scelta, nell'ordine del menu.
pub fn protocol_templates() -> [Protocol; 9] {
    [
        Protocol::Uart(UartConfig::default()),
        Protocol::I2c,
        Protocol::Can(CanConfig { bitrate: 500_000.0, inverted: false }),
        Protocol::OneWire,
        Protocol::Manchester { bit_period_s: None, ieee: false },
        Protocol::Pwm,
        Protocol::Servo,
        Protocol::IrNec,
        Protocol::IrRc5,
    ]
}

/// Scelta usata per calcolare le annotazioni: protocollo completo, canale e soglia.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub protocol: Protocol,
    /// 0 per CH1, 1 per CH2.
    pub channel: usize,
    /// Soglia in volt; `None` per quella automatica.
    pub threshold: Option<f32>,
}

/// Annotazioni calcolate per una sequenza di frame e una scelta.
#[derive(Debug, Clone, PartialEq)]
struct Cached {
    sequence: u64,
    choice: Choice,
    result: Result<Vec<Annotation>, String>,
}

/// Stato del pannello dei decoder.
pub struct DecodeState {
    pub enabled: bool,
    /// Indice in `protocol_templates`.
    pub selected: usize,
    /// 0 per CH1, 1 per CH2.
    pub channel: usize,
    pub threshold_auto: bool,
    pub threshold_volts: f32,
    pub uart_auto_baud: bool,
    pub uart_baud: f64,
    pub uart_data_bits: u8,
    pub uart_parity: Parity,
    pub uart_stop_bits: f32,
    pub uart_inverted: bool,
    pub can_bitrate: f64,
    pub can_inverted: bool,
    cache: Option<Cached>,
}

impl Default for DecodeState {
    fn default() -> Self {
        DecodeState {
            enabled: false,
            selected: 0,
            channel: 0,
            threshold_auto: true,
            threshold_volts: 1.5,
            uart_auto_baud: true,
            uart_baud: 9600.0,
            uart_data_bits: 8,
            uart_parity: Parity::None,
            uart_stop_bits: 1.0,
            uart_inverted: false,
            can_bitrate: 500_000.0,
            can_inverted: false,
            cache: None,
        }
    }
}

impl DecodeState {
    /// Protocollo con i parametri correnti.
    fn protocol(&self) -> Protocol {
        let template = protocol_templates().get(self.selected).copied().unwrap_or(protocol_templates()[0]);
        match template {
            Protocol::Uart(_) => Protocol::Uart(self.uart_config()),
            Protocol::Can(_) => Protocol::Can(CanConfig { bitrate: self.can_bitrate, inverted: self.can_inverted }),
            other => other,
        }
    }

    fn uart_config(&self) -> UartConfig {
        UartConfig {
            baud: if self.uart_auto_baud { None } else { Some(self.uart_baud) },
            data_bits: self.uart_data_bits,
            parity: self.uart_parity,
            stop_bits: self.uart_stop_bits,
            lsb_first: true,
            inverted: self.uart_inverted,
        }
    }

    fn choice(&self) -> Choice {
        Choice {
            protocol: self.protocol(),
            channel: self.channel,
            threshold: if self.threshold_auto { None } else { Some(self.threshold_volts) },
        }
    }
}

/// Decodifica il frame con la scelta data. Funzione pura: nessuno stato.
pub fn compute(frame: &Frame, choice: &Choice) -> Result<Vec<Annotation>, String> {
    let (primary, secondary) = if choice.channel == 0 {
        (&frame.ch1, frame.ch2.as_ref())
    } else {
        let ch2 = frame.ch2.as_ref().ok_or_else(|| "il canale 2 non è attivo".to_string())?;
        (ch2, Some(&frame.ch1))
    };
    protocol::decode(&choice.protocol, primary, secondary, choice.threshold)
}

/// Ricalcola solo se cambiano la sequenza del frame o la scelta.
fn refresh(cache: &mut Option<Cached>, frame: &Frame, choice: &Choice) {
    if let Some(cached) = cache {
        if cached.sequence == frame.sequence && cached.choice == *choice {
            return;
        }
    }
    *cache = Some(Cached { sequence: frame.sequence, choice: choice.clone(), result: compute(frame, choice) });
}

/// Controlli del decoder, con la spiegazione di ciascuno.
pub fn controls(ui: &mut Ui, ui_state: &UiState, state: &mut DecodeState) {
    CollapsingHeader::new("Decoder").default_open(true).show(ui, |ui| {
        let enable = ui.checkbox(&mut state.enabled, "Attiva");
        help::attach(enable, HelpId::DecodeProtocol, ui_state);

        let templates = protocol_templates();
        ui.horizontal(|ui| {
            ui.label("Protocollo");
            let selected = templates.get(state.selected).map_or("—", |p| p.name());
            let resp = ComboBox::from_id_salt("decode_protocol")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (i, p) in templates.iter().enumerate() {
                        ui.selectable_value(&mut state.selected, i, p.name());
                    }
                })
                .response;
            help::attach(resp, HelpId::DecodeProtocol, ui_state);
        });

        ui.horizontal(|ui| {
            ui.label("Canale");
            let resp = ComboBox::from_id_salt("decode_channel")
                .selected_text(format!("CH{}", state.channel + 1))
                .show_ui(ui, |ui| {
                    for ch in 0..2 {
                        ui.selectable_value(&mut state.channel, ch, format!("CH{}", ch + 1));
                    }
                })
                .response;
            help::attach(resp, HelpId::DecodeChannel, ui_state);
        });

        ui.horizontal(|ui| {
            let auto = ui.checkbox(&mut state.threshold_auto, "Soglia automatica");
            help::attach(auto, HelpId::DecodeThreshold, ui_state);
            let volts = DragValue::new(&mut state.threshold_volts)
                .speed(0.01)
                .custom_formatter(|v, _| format_volts(v as f32));
            let drag = ui.add_enabled(!state.threshold_auto, volts);
            help::attach(drag, HelpId::DecodeThreshold, ui_state);
        });

        match state.protocol() {
            Protocol::Uart(_) => uart_controls(ui, ui_state, state),
            Protocol::Can(_) => can_controls(ui, ui_state, state),
            _ => {}
        }
    });
}

fn uart_controls(ui: &mut Ui, ui_state: &UiState, state: &mut DecodeState) {
    ui.horizontal(|ui| {
        let auto = ui.checkbox(&mut state.uart_auto_baud, "Baud automatico");
        help::attach(auto, HelpId::DecodeUartBaud, ui_state);
        let baud = DragValue::new(&mut state.uart_baud).speed(10.0).suffix(" bit/s");
        let drag = ui.add_enabled(!state.uart_auto_baud, baud);
        help::attach(drag, HelpId::DecodeUartBaud, ui_state);
    });
    ui.horizontal(|ui| {
        ui.label("Bit dati");
        let bits = ui.add(DragValue::new(&mut state.uart_data_bits).range(5..=9));
        help::attach(bits, HelpId::DecodeUartFormat, ui_state);
        let resp = ComboBox::from_id_salt("decode_parity")
            .selected_text(parity_name(state.uart_parity))
            .show_ui(ui, |ui| {
                for p in [Parity::None, Parity::Even, Parity::Odd] {
                    ui.selectable_value(&mut state.uart_parity, p, parity_name(p));
                }
            })
            .response;
        help::attach(resp, HelpId::DecodeUartFormat, ui_state);
    });
    ui.horizontal(|ui| {
        ui.label("Stop");
        let resp = ComboBox::from_id_salt("decode_stop")
            .selected_text(stop_name(state.uart_stop_bits))
            .show_ui(ui, |ui| {
                for s in [1.0_f32, 1.5, 2.0] {
                    ui.selectable_value(&mut state.uart_stop_bits, s, stop_name(s));
                }
            })
            .response;
        help::attach(resp, HelpId::DecodeUartFormat, ui_state);
    });
    let inverted = ui.checkbox(&mut state.uart_inverted, "Linea invertita");
    help::attach(inverted, HelpId::DecodeInverted, ui_state);
}

fn can_controls(ui: &mut Ui, ui_state: &UiState, state: &mut DecodeState) {
    ui.horizontal(|ui| {
        ui.label("Bitrate");
        let rate = ui.add(DragValue::new(&mut state.can_bitrate).speed(1000.0).range(1_000.0..=1_000_000.0).suffix(" bit/s"));
        help::attach(rate, HelpId::DecodeCanBitrate, ui_state);
    });
    let inverted = ui.checkbox(&mut state.can_inverted, "Linea invertita");
    help::attach(inverted, HelpId::DecodeInverted, ui_state);
}

fn parity_name(parity: Parity) -> &'static str {
    match parity {
        Parity::None => "Nessuna",
        Parity::Even => "Pari",
        Parity::Odd => "Dispari",
    }
}

fn stop_name(stop_bits: f32) -> &'static str {
    if stop_bits < 1.25 {
        "1"
    } else if stop_bits < 1.75 {
        "1,5"
    } else {
        "2"
    }
}

/// Striscia delle annotazioni sotto il display, allineata in tempo con `display`,
/// e riga di riepilogo. Lo spazio viene allocato qui: il chiamante lo ha già sottratto al display.
pub fn strip(ui: &mut Ui, display: Rect, frame: Option<&Frame>, state: &mut DecodeState, ui_state: &UiState) {
    let (response, painter) = ui.allocate_painter(vec2(display.width(), RESERVED_HEIGHT), Sense::hover());
    let Some(frame) = frame else {
        return;
    };
    let strip = Rect::from_min_size(pos2(display.left(), response.rect.top()), vec2(display.width(), STRIP_HEIGHT));
    let choice = state.choice();
    refresh(&mut state.cache, frame, &choice);
    let Some(cached) = &state.cache else {
        return;
    };
    match &cached.result {
        Err(message) => {
            painter.text(strip.left_center(), Align2::LEFT_CENTER, message, FontId::proportional(13.0), Color32::RED);
        }
        Ok(annotations) => {
            for annotation in annotations {
                draw_annotation(&painter, display, strip, annotation, ui_state);
            }
            let text = protocol::summary(annotations);
            painter.text(
                pos2(display.left() + 4.0, strip.bottom() + 2.0),
                Align2::LEFT_TOP,
                text,
                FontId::proportional(12.0),
                TEXT,
            );
        }
    }
}

fn draw_annotation(painter: &Painter, display: Rect, strip: Rect, annotation: &Annotation, ui_state: &UiState) {
    let raw_left = time_to_x(display, annotation.start_s, ui_state);
    let raw_right = time_to_x(display, annotation.end_s, ui_state);
    if raw_right < display.left() || raw_left > display.right() {
        return;
    }
    let left = raw_left.max(display.left());
    let right = raw_right.min(display.right()).max(left + MIN_WIDTH);
    let rect = Rect::from_min_max(pos2(left, strip.top()), pos2(right, strip.bottom()));
    let (fill, ink) = if annotation.error {
        (Color32::RED, Color32::WHITE)
    } else {
        (kind_color(annotation.kind), Color32::BLACK)
    };
    painter.rect_filled(rect, 3.0, fill);
    // Il testo compare solo se la stima della sua larghezza ci sta nel rettangolo.
    let needed = annotation.text.chars().count() as f32 * CHAR_WIDTH + 6.0;
    if needed <= rect.width() {
        painter.text(
            pos2(rect.left() + 3.0, rect.center().y),
            Align2::LEFT_CENTER,
            &annotation.text,
            FontId::monospace(11.0),
            ink,
        );
    }
}

fn kind_color(kind: &str) -> Color32 {
    match kind {
        "data" => Color32::from_rgb(120, 190, 255),
        "address" => Color32::from_rgb(250, 180, 60),
        "id" => Color32::from_rgb(190, 140, 250),
        "ack" => Color32::from_rgb(90, 200, 120),
        "nack" => Color32::from_rgb(230, 120, 120),
        "start" | "stop" => Color32::from_gray(150),
        "command" => Color32::from_rgb(80, 220, 200),
        "crc" => Color32::from_rgb(240, 220, 80),
        "reset" | "presence" => Color32::from_rgb(180, 130, 90),
        _ => Color32::from_rgb(170, 170, 200),
    }
}

#[cfg(test)]
mod tests {
    use h6022_core::{EngineConfig, Frame, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_decode::Protocol;
    use h6022_decode::uart::UartConfig;
    use h6022_dsp::Waveform;
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::{Choice, compute, refresh};

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

    fn wave(samples: Vec<f32>) -> Waveform {
        Waveform { samples, sample_rate: 1e6, t0: 0.0 }
    }

    fn frame(sequence: u64, ch1: Vec<f32>, ch2: Option<Vec<f32>>) -> Frame {
        Frame {
            sequence,
            ch1: wave(ch1),
            ch2: ch2.map(wave),
            triggered: true,
            trigger_sample: Some(0),
            config: config(),
        }
    }

    fn square() -> Vec<f32> {
        (0..100).map(|i| if (i / 10) % 2 == 0 { 1.0 } else { -1.0 }).collect()
    }

    fn choice(protocol: Protocol) -> Choice {
        Choice { protocol, channel: 0, threshold: None }
    }

    #[test]
    fn cache_recomputes_only_when_sequence_or_choice_changes() {
        let pwm = choice(Protocol::Pwm);
        let uart = choice(Protocol::Uart(UartConfig::default()));
        let on_square = frame(1, square(), None);
        let flat = frame(1, vec![0.0; 100], None);
        assert_ne!(compute(&on_square, &pwm), compute(&flat, &pwm), "i due frame devono dare esiti diversi");

        let mut cache = None;
        refresh(&mut cache, &on_square, &pwm);
        let first = cache.as_ref().expect("cache riempita").result.clone();
        assert_eq!(first, compute(&on_square, &pwm));

        // Stessa sequenza e stessa scelta: nessun ricalcolo, il risultato resta quello di prima.
        refresh(&mut cache, &flat, &pwm);
        assert_eq!(cache.as_ref().expect("cache").result, first);

        // Protocollo diverso: si ricalcola sul frame attuale.
        refresh(&mut cache, &flat, &uart);
        assert_eq!(cache.as_ref().expect("cache ricalcolata").result, compute(&flat, &uart));
    }

    #[test]
    fn i2c_with_one_channel_is_an_error() {
        let i2c = choice(Protocol::I2c);
        let single = frame(1, square(), None);
        assert!(compute(&single, &i2c).is_err());
        let on_second = Choice { channel: 1, ..i2c };
        assert!(compute(&single, &on_second).is_err());
    }
}
