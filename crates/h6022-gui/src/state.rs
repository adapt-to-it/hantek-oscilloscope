//! Stato dell'interfaccia, separato dalla configurazione del motore.

use eframe::egui::Color32;
use h6022_dsp::Window;
use h6022_usb::Channel;

pub const CH_COLORS: [Color32; 2] = [Color32::from_rgb(250, 210, 40), Color32::from_rgb(60, 200, 250)];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelView {
    pub enabled: bool,
    /// Volt per divisione all'ingresso della sonda.
    pub volts_per_div: f32,
    /// Posizione verticale dello zero, in divisioni dal centro (positivo = in alto).
    pub offset_div: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorMode {
    Off,
    Time,
    Voltage,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursors {
    pub mode: CursorMode,
    /// Tempi dei due cursori verticali, in secondi relativi al trigger.
    pub t: [f64; 2],
    /// Tensioni dei due cursori orizzontali, in volt del canale `channel`.
    pub v: [f32; 2],
    pub channel: Channel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Time,
    Spectrum,
    Xy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub channels: [ChannelView; 2],
    /// Secondi per divisione.
    pub time_per_div: f64,
    /// Spostamento orizzontale della vista, in secondi (positivo = la vista va a destra).
    pub horizontal_offset: f64,
    pub cursors: Cursors,
    pub view: View,
    pub window: Window,
    /// Spettro: asse delle frequenze logaritmico.
    pub spectrum_log_freq: bool,
    /// Mostra le spiegazioni al passaggio del puntatore (tasto `i`).
    pub help: bool,
    /// Persistenza: le tracce precedenti sfumano invece di sparire.
    pub persistence: bool,
}

impl Default for UiState {
    fn default() -> Self {
        let ch = ChannelView { enabled: true, volts_per_div: 1.0, offset_div: 0.0 };
        UiState {
            channels: [ChannelView { offset_div: 0.0, ..ch }, ChannelView { offset_div: -3.0, ..ch }],
            time_per_div: 100e-6,
            horizontal_offset: 0.0,
            cursors: Cursors { mode: CursorMode::Off, t: [-100e-6, 100e-6], v: [0.0, 1.0], channel: Channel::Ch1 },
            view: View::Time,
            window: Window::FlatTop,
            spectrum_log_freq: true,
            help: false,
            persistence: false,
        }
    }
}
