//! Server locale dell'oscilloscopio su `127.0.0.1`.
//!
//! - `/mcp`: Model Context Protocol, trasporto HTTP streamable.
//! - `/ws`: WebSocket binario con i frame del motore (formato in `ws`).
//! - `/health`: `{"ok":true,"version":"..."}`.
//!
//! Il server gira in un runtime tokio su un thread proprio: GUI e CLI restano sincrone.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use h6022_core::EngineHandle;
use h6022_record::Recorder;

pub mod api;
pub mod http;
pub mod tools;
pub mod ws;

pub use http::{ServerHandle, spawn};

/// Porta predefinita: 6022, come il modello.
pub const DEFAULT_PORT: u16 = 6022;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    pub addr: SocketAddr,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig { addr: SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)) }
    }
}

/// Stato condiviso da MCP e WebSocket.
#[derive(Clone)]
pub struct AppState {
    pub engine: EngineHandle,
    /// "hardware" o "simulator".
    pub source: &'static str,
    /// Indirizzo effettivo dopo il bind, per comporre `websocket_url`.
    pub addr: SocketAddr,
    pub recorder: Arc<Mutex<Option<Recorder>>>,
}
