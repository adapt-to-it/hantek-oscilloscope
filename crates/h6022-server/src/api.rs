//! Parametri e risultati degli strumenti MCP. Tensioni in volt, tempi in secondi, frequenze in Hz.
//! I canali si indicano con 1 o 2.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CouplingName {
    Dc,
    Ac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ModeName {
    Auto,
    Normal,
    Single,
    Roll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlopeName {
    Rising,
    Falling,
    Either,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogicOpName {
    And,
    Or,
}

/// Condizione di un canale nel trigger logico: un fronte, un livello rispetto alla soglia o nessuna.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CondName {
    Rising,
    Falling,
    Either,
    High,
    Low,
    Ignore,
}

/// Trigger logico: una condizione per canale combinate con `op`.
/// Il canale sorgente usa `trigger_level_v`, l'altro `other_level_v`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LogicTrigger {
    pub op: LogicOpName,
    pub ch1: CondName,
    pub ch2: CondName,
    pub other_level_v: f32,
}

/// Trigger A → B: fronte A sul canale sorgente a `trigger_level_v`, poi il primo fronte B
/// sull'altro canale a `other_level_v` che arriva almeno `delay_s` secondi dopo A.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SequenceTrigger {
    pub slope_a: SlopeName,
    pub slope_b: SlopeName,
    pub other_level_v: f32,
    pub delay_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WindowName {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
    FlatTop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FormatName {
    Csv,
    Npy,
    Wav,
}

/// Configurazione completa del motore, come la vede un client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigView {
    /// Frequenza di campionamento effettiva.
    pub sample_rate_hz: f64,
    /// 1 o 2 canali acquisiti.
    pub channels: u8,
    /// Portata per canale: 5, 2.5, 1 o 0.5 (±V).
    pub range_v: [f32; 2],
    pub coupling: [CouplingName; 2],
    /// Attenuazione della sonda: 1 o 10.
    pub probe: [f32; 2],
    pub mode: ModeName,
    /// Campioni per canale in ogni frame.
    pub record_length: usize,
    /// Frazione 0..1 del frame prima del trigger.
    pub pretrigger: f32,
    pub trigger_source: u8,
    /// "edge", "pulse_width", "runt", "window", "timeout", "pattern", "logic" o "sequence".
    pub trigger_kind: String,
    /// Solo per i trigger "edge" e "timeout".
    pub trigger_slope: Option<SlopeName>,
    /// Solo per il trigger "logic".
    pub trigger_logic: Option<LogicTrigger>,
    /// Solo per il trigger "sequence".
    pub trigger_sequence: Option<SequenceTrigger>,
    pub trigger_level_v: f32,
    pub trigger_hysteresis_v: f32,
    pub trigger_holdoff_s: f64,
}

/// Modifica parziale: i campi assenti restano invariati.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigPatch {
    /// Frequenza desiderata; il dispositivo usa la più vicina disponibile.
    pub sample_rate_hz: Option<f64>,
    /// 1 o 2.
    pub channels: Option<u8>,
    /// Portata del canale 1: 5, 2.5, 1 o 0.5.
    pub range1_v: Option<f32>,
    pub range2_v: Option<f32>,
    pub coupling1: Option<CouplingName>,
    pub coupling2: Option<CouplingName>,
    /// 1 o 10.
    pub probe1: Option<f32>,
    pub probe2: Option<f32>,
    pub mode: Option<ModeName>,
    pub record_length: Option<usize>,
    pub pretrigger: Option<f32>,
    /// 1 o 2.
    pub trigger_source: Option<u8>,
    /// Imposta un trigger a fronte con questa pendenza.
    /// `trigger_slope`, `trigger_logic` e `trigger_sequence` si escludono a vicenda.
    pub trigger_slope: Option<SlopeName>,
    /// Imposta un trigger logico. Richiede due canali.
    pub trigger_logic: Option<LogicTrigger>,
    /// Imposta un trigger A → B. Richiede due canali.
    pub trigger_sequence: Option<SequenceTrigger>,
    pub trigger_level_v: Option<f32>,
    pub trigger_hysteresis_v: Option<f32>,
    pub trigger_holdoff_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordingView {
    pub path: String,
    pub format: FormatName,
    pub frames: u64,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusView {
    /// "hardware" o "simulator".
    pub source: String,
    /// "running", "armed" o "stopped".
    pub state: String,
    pub frames: u64,
    pub triggers: u64,
    pub samples: u64,
    pub sample_rate_measured_hz: f64,
    pub config: ConfigView,
    pub recording: Option<RecordingView>,
    /// Per esempio "ws://127.0.0.1:6022/ws".
    pub websocket_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CaptureParams {
    /// Attesa massima del trigger. Predefinito 5 s, massimo 60 s.
    pub timeout_s: Option<f64>,
    /// Punti min/max per canale nella risposta. Predefinito 500, massimo 5000.
    pub points: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotParams {
    /// Punti min/max per canale. Predefinito 500, massimo 5000.
    pub points: Option<usize>,
}

/// Una traccia ridotta a `points` coppie min/max su intervalli di tempo uguali.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TraceView {
    pub channel: u8,
    pub min_v: Vec<f32>,
    pub max_v: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FrameView {
    pub sequence: u64,
    pub triggered: bool,
    pub sample_rate_hz: f64,
    pub record_length: usize,
    /// Tempo del primo e dell'ultimo campione rispetto al trigger.
    pub t_start_s: f64,
    pub t_end_s: f64,
    pub points: usize,
    pub channels: Vec<TraceView>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeasureParams {
    /// Frame da mediare. Predefinito 1, massimo 50.
    pub frames: Option<u32>,
    /// Attesa massima per tutti i frame. Predefinito 5 s, massimo 60 s.
    pub timeout_s: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChannelMeasures {
    pub channel: u8,
    pub vpp: f32,
    pub vmax: f32,
    pub vmin: f32,
    pub vmean: f32,
    pub vrms: f32,
    pub vac_rms: f32,
    pub vtop: f32,
    pub vbase: f32,
    pub vamp: f32,
    pub frequency_hz: Option<f64>,
    pub period_s: Option<f64>,
    /// Percentuale 0..100.
    pub duty_pct: Option<f64>,
    pub pulse_width_pos_s: Option<f64>,
    pub pulse_width_neg_s: Option<f64>,
    pub rise_s: Option<f64>,
    pub fall_s: Option<f64>,
    pub overshoot_pct: Option<f32>,
    pub preshoot_pct: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeasureView {
    pub frames: u32,
    pub channels: Vec<ChannelMeasures>,
    /// Sfasamento di CH2 rispetto a CH1, positivo se CH2 è in ritardo.
    pub phase_deg: Option<f64>,
    pub delay_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpectrumParams {
    /// 1 o 2.
    pub channel: u8,
    /// Predefinita flat_top.
    pub window: Option<WindowName>,
    /// Numero di picchi. Predefinito 10, massimo 50.
    pub peaks: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PeakView {
    pub frequency_hz: f64,
    pub vrms: f32,
    pub dbv: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpectrumView {
    pub channel: u8,
    pub window: WindowName,
    /// Larghezza di un bin.
    pub bin_hz: f64,
    pub peaks: Vec<PeakView>,
    /// Distorsione armonica rispetto al picco più alto.
    pub thd_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordParams {
    /// Percorso assoluto, oppure relativo alla cartella ~/h6022-recordings.
    pub path: String,
    /// Predefinito dall'estensione del percorso.
    pub format: Option<FormatName>,
    pub max_frames: Option<u64>,
    pub max_seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecordSummaryView {
    pub path: String,
    pub format: FormatName,
    pub frames: u64,
    pub channels: u8,
    pub sample_rate_hz: f64,
    pub samples_per_channel: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// 1 o 2.
    pub channel: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SignalKind {
    /// Livello costante.
    Dc,
    /// Solo rumore, nessun fronte.
    Noise,
    /// Onda quadra con duty tra 45 e 55 %.
    Square,
    /// Onda rettangolare con altro duty.
    Pwm,
    Sine,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisView {
    pub channel: u8,
    pub kind: SignalKind,
    /// Una frase in italiano, per esempio "Onda quadra 1,00 kHz, 0–3,15 V, duty 50 %".
    pub summary: String,
    /// Base dei tempi che mostra circa 3 periodi.
    pub suggested_time_per_div_s: Option<f64>,
    /// Portata più piccola che contiene il segnale.
    pub suggested_range_v: f32,
    pub measures: ChannelMeasures,
    pub thd_pct: Option<f64>,
}
