//! Stato condiviso dell'applicazione, gestito da Tauri.

use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

use h6022_core::{EngineHandle, TriggerMode};
use h6022_record::Recorder;
use h6022_server::ServerHandle;
use h6022_view::settings::ViewSettings;
use h6022_view::tools::ToolSettings;

pub struct AppState {
    pub engine: EngineHandle,
    /// "hardware" o "simulator".
    pub source: &'static str,
    /// Versione del firmware; `None` con il simulatore.
    pub firmware: Option<u16>,
    pub view: Arc<Mutex<ViewSettings>>,
    /// Filtro e traccia matematica applicati al display.
    pub tools: Arc<Mutex<ToolSettings>>,
    /// Colonne del display chieste dal frontend.
    pub columns: Arc<AtomicUsize>,
    /// Cresce a ogni `subscribe_display`: il thread di invio di una sottoscrizione vecchia si ferma.
    pub display_generation: Arc<AtomicU64>,
    /// Modo di trigger da ripristinare con Run dopo un Single.
    pub previous_mode: Mutex<Option<TriggerMode>>,
    pub server: Mutex<Option<ServerHandle>>,
    /// Indirizzo MCP o motivo per cui il server non è partito.
    pub server_status: Mutex<String>,
    /// Registrazione avviata dal frontend; resta qui anche dopo un limite raggiunto, fino a `stop_recording`.
    pub recorder: Mutex<Option<Recorder>>,
}
