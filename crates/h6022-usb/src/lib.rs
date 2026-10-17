//! Driver USB per l'oscilloscopio Hantek 6022BE.
//!
//! Il crate carica il firmware nel Cypress FX2LP, invia i comandi vendor,
//! legge la calibrazione dalla EEPROM e riceve i campioni in streaming bulk.

pub mod calibration;
pub mod device;
pub mod error;
pub mod firmware;
pub mod ihex;
pub mod nusb_transport;
pub mod rates;
pub mod stream;
pub mod transport;
pub mod types;

pub use calibration::Calibration;
pub use device::{AcquisitionConfig, Hantek6022};
pub use error::{Error, Result};
pub use stream::{Block, Stream, StreamConfig, StreamStats};
pub use transport::{BulkSource, Transport};
pub use types::*;
