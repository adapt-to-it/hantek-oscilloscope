//! Motore di acquisizione dell'oscilloscopio.

pub mod engine;
pub mod hardware;
pub mod sim;
pub mod source;
pub mod trigger;
pub mod types;

pub use engine::{Engine, EngineHandle};
pub use source::{SampleSource, VoltBlock};
pub use types::*;
