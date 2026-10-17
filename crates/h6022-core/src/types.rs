//! Tipi condivisi del motore.

use h6022_dsp::Waveform;
use h6022_usb::{AcquisitionConfig, Channel};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Device(#[from] h6022_usb::Error),
    #[error("motore fermo")]
    Stopped,
    #[error("configurazione non valida: {0}")]
    InvalidConfig(String),
}

pub type CoreResult<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slope {
    Rising,
    Falling,
    Either,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    Positive,
    Negative,
}

/// Condizione sulla durata, in secondi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidthCondition {
    LessThan(f64),
    GreaterThan(f64),
    Within(f64, f64),
    Outside(f64, f64),
}

/// Livello logico richiesto su un canale nel trigger a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    High,
    Low,
    DontCare,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerKind {
    /// Attraversamento di `level`.
    Edge { slope: Slope },
    /// Impulso di `polarity` rispetto a `level` la cui durata soddisfa `condition`.
    /// Il punto di trigger è la fine dell'impulso.
    PulseWidth { polarity: Polarity, condition: WidthCondition },
    /// Impulso che supera `low` ma torna indietro senza superare `high`.
    Runt { polarity: Polarity, low: f32, high: f32 },
    /// Il segnale entra (`enter` = true) o esce dalla fascia low..high.
    Window { low: f32, high: f32, enter: bool },
    /// Nessun fronte di `slope` per almeno `seconds`.
    Timeout { slope: Slope, seconds: f64 },
    /// Entrambi i canali confrontati con `level`; scatta quando il pattern diventa vero.
    Pattern { ch1: Level, ch2: Level },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriggerConfig {
    pub source: Channel,
    pub kind: TriggerKind,
    /// Soglia in volt.
    pub level: f32,
    /// Isteresi in volt attorno a `level`, contro i falsi trigger del rumore.
    pub hysteresis: f32,
    /// Tempo minimo tra due trigger, in secondi.
    pub holdoff: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerMode {
    /// Frame non sincronizzato se il trigger manca per `AUTO_TIMEOUT`.
    Auto,
    /// Solo frame sincronizzati.
    Normal,
    /// Un frame sincronizzato, poi stato `Stopped`.
    Single,
    /// Nessun trigger: frame continui degli ultimi campioni.
    Roll,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    pub acquisition: AcquisitionConfig,
    pub trigger: TriggerConfig,
    pub mode: TriggerMode,
    /// Campioni per canale in ogni frame.
    pub record_length: usize,
    /// Frazione 0..1 del frame prima del punto di trigger.
    pub pretrigger: f32,
    /// Attenuazione della sonda per canale (1 o 10).
    pub probe: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunState {
    Running,
    /// In attesa di trigger (Normal o Single).
    Armed,
    Stopped,
}

/// Frame pronto per display, misure e client.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Numero progressivo dall'avvio del motore.
    pub sequence: u64,
    pub ch1: Waveform,
    /// `None` in modalità a un canale.
    pub ch2: Option<Waveform>,
    pub triggered: bool,
    /// Indice assoluto del campione di trigger dall'ultima configurazione.
    pub trigger_sample: Option<u64>,
    pub config: EngineConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EngineStats {
    pub frames: u64,
    pub triggers: u64,
    pub samples: u64,
    /// Campioni al secondo ricevuti, media sull'ultimo secondo.
    pub sample_rate_measured: f64,
}
