//! Parametri e risultati degli strumenti MCP dei decoder. Tempi in secondi, tensioni in volt.
//! I canali si indicano con 1 o 2.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ParityName {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecodeParams {
    /// uart, i2c, can, onewire, manchester, pwm, servo, ir_nec o ir_rc5.
    pub protocol: String,
    /// 1 o 2. Per i2c è SCL; SDA è l'altro canale, che richiede la modalità a due canali.
    pub channel: u8,
    /// Soglia in volt; assente: soglia automatica.
    pub threshold_v: Option<f32>,
    /// uart: velocità in baud; assente: stima automatica.
    pub baud: Option<f64>,
    /// uart: bit di dato, da 5 a 9. Predefinito 8.
    pub data_bits: Option<u8>,
    /// uart: parità. Predefinito none.
    pub parity: Option<ParityName>,
    /// uart: bit di stop, 1, 1.5 o 2. Predefinito 1.
    pub stop_bits: Option<f32>,
    /// uart e can: linea invertita. Predefinito false.
    pub inverted: Option<bool>,
    /// can: velocità in bit al secondo, obbligatoria.
    pub bitrate: Option<f64>,
    /// manchester: periodo di bit in secondi; assente: stima automatica.
    pub bit_period_s: Option<f64>,
    /// manchester: convenzione IEEE. Predefinito true.
    pub ieee: Option<bool>,
    /// Numero massimo di annotazioni restituite, da 1 a 5000. Predefinito 200.
    pub max_annotations: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnnotationView {
    pub start_s: f64,
    pub end_s: f64,
    /// Categoria breve: data, start, stop, address, ack, nack, error e altre.
    pub kind: String,
    /// Testo da mostrare, per esempio "0x41 'A'".
    pub text: String,
    /// Valore numerico, se c'è.
    pub value: Option<u64>,
    /// Vero per errori di framing, parità, CRC o temporizzazione.
    pub error: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DecodeView {
    pub protocol: String,
    pub channel: u8,
    pub annotations: Vec<AnnotationView>,
    /// Vero se le annotazioni sono più di `max_annotations`.
    pub truncated: bool,
    /// Riepilogo dei byte decodificati e degli errori.
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChannelParams {
    /// 1 o 2.
    pub channel: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClassificationView {
    /// Classe in minuscolo: dc, noise, analog, clock, pwm, serial, burst, other.
    pub class: String,
    /// 0..1, quanto i criteri sono rispettati.
    pub confidence: f32,
    /// Una frase in italiano con i numeri chiave.
    pub summary: String,
    /// Baud stimato, per i segnali seriali.
    pub baud_estimate: Option<f64>,
    /// Decoder suggeriti, per esempio uart o manchester.
    pub suggested_decoders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SegmentsParams {
    /// 1 o 2.
    pub channel: u8,
    /// Pausa minima in secondi che separa due pacchetti; assente: stima automatica.
    pub min_gap_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SegmentView {
    pub start_s: f64,
    pub end_s: f64,
    pub transitions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SegmentsView {
    pub segments: Vec<SegmentView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PatternParams {
    /// 1 o 2.
    pub channel: u8,
    /// Inizio del campione in secondi, nel frame.
    pub start_s: f64,
    /// Fine del campione in secondi, nel frame.
    pub end_s: f64,
    /// Correlazione minima tra -1 e 1. Predefinito 0.8.
    pub min_score: Option<f32>,
    /// Numero massimo di match, da 1 a 200. Predefinito 20.
    pub max_matches: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MatchView {
    pub time_s: f64,
    /// Correlazione normalizzata tra -1 e 1.
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MatchesView {
    pub matches: Vec<MatchView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormatName {
    Vcd,
    Sr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExportParams {
    /// Percorso assoluto, oppure relativo alla cartella ~/h6022-recordings.
    pub path: String,
    /// vcd o sr. Le annotazioni dei decoder entrano solo nel vcd.
    pub format: ExportFormatName,
    /// Decoder da includere come annotazioni nel vcd; stessi campi di `decode`.
    pub decode: Option<DecodeParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExportView {
    pub path: String,
    pub bytes: u64,
}
