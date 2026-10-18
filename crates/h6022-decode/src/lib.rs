//! Decoder e analisi su forme d'onda in volt.
//!
//! Ogni decoder lavora su una o due `LogicTrace` ottenute con `logic::digitize` e
//! restituisce `Annotation` ordinate per `start_s`. I tempi seguono `Waveform::t0`:
//! il campione `i` vale `t0 + i / sample_rate` secondi.

pub mod can;
pub mod classify;
pub mod export;
pub mod i2c;
pub mod ir;
pub mod logic;
pub mod manchester;
pub mod onewire;
pub mod protocol;
pub mod pwm;
pub mod search;
pub mod uart;

pub use logic::{LogicTrace, digitize};
pub use protocol::Protocol;

/// Un elemento decodificato: un byte, un bit di start, un indirizzo, un errore.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub start_s: f64,
    pub end_s: f64,
    /// Categoria breve in minuscolo: "data", "start", "stop", "address", "ack", "nack",
    /// "id", "crc", "command", "bit", "reset", "presence", "error".
    pub kind: &'static str,
    /// Testo da mostrare, per esempio "0x41 'A'".
    pub text: String,
    /// Valore numerico, se c'è.
    pub value: Option<u64>,
    /// Vero per errori di framing, parità, CRC o temporizzazione.
    pub error: bool,
}

impl Annotation {
    pub fn new(start_s: f64, end_s: f64, kind: &'static str, text: impl Into<String>) -> Self {
        Annotation { start_s, end_s, kind, text: text.into(), value: None, error: false }
    }

    pub fn with_value(mut self, value: u64) -> Self {
        self.value = Some(value);
        self
    }

    pub fn as_error(mut self) -> Self {
        self.error = true;
        self
    }
}
