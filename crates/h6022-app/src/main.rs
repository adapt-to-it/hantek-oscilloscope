//! Applicazione desktop Tauri: motore, server MCP e comandi per il frontend in `ui/`.
//!
//! `--sim` usa il simulatore; `--no-server` non avvia il server MCP; `H6022_PORT` cambia la porta;
//! `H6022_MODE` apre l'app sulla vista indicata.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

use h6022_core::hardware::HardwareSource;
use h6022_core::sim::SimulatedSource;
use h6022_core::{Engine, SampleSource};
use h6022_view::settings::ViewSettings;
use tauri::Manager;

use crate::state::AppState;

fn main() {
    let simulated = std::env::args().any(|a| a == "--sim");
    let with_server = !std::env::args().any(|a| a == "--no-server");
    let port = std::env::var("H6022_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(h6022_server::DEFAULT_PORT);
    let (source, name): (Box<dyn SampleSource>, &'static str) = if simulated {
        (Box::new(SimulatedSource::bench_default()), "simulator")
    } else {
        match HardwareSource::open() {
            Ok(source) => (Box::new(source), "hardware"),
            Err(e) => {
                eprintln!("dispositivo non disponibile ({e}): uso il simulatore");
                (Box::new(SimulatedSource::bench_default()), "simulator")
            }
        }
    };
    let view = ViewSettings::default();
    let engine = Engine::start(source, view.initial_config()).expect("avvio del motore");

    let (server, server_status) = if with_server {
        let config = h6022_server::ServerConfig { addr: std::net::SocketAddr::from(([127, 0, 0, 1], port)) };
        match h6022_server::spawn(engine.clone(), name, config) {
            Ok(server) => {
                let status = format!("MCP http://{}/mcp", server.addr());
                (Some(server), status)
            }
            Err(e) => (None, format!("MCP non avviato: {e}")),
        }
    } else {
        (None, "MCP disattivato".to_string())
    };

    let state = AppState {
        engine,
        source: name,
        view: Arc::new(Mutex::new(view)),
        columns: Arc::new(AtomicUsize::new(1000)),
        display_generation: Arc::new(AtomicU64::new(0)),
        previous_mode: Mutex::new(None),
        server: Mutex::new(server),
        server_status: Mutex::new(server_status),
    };

    let app = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::get_view,
            commands::set_view,
            commands::run_command,
            commands::set_columns,
            commands::subscribe_display,
            commands::get_trigger_level,
            commands::set_trigger_level,
            commands::get_acquisition,
            commands::set_acquisition,
            commands::measurements,
            commands::spectrum,
            commands::xy,
            commands::initial_mode,
            commands::decode,
        ])
        .setup(|app| {
            // SIGINT e SIGTERM chiudono l'app con la stessa sequenza della chiusura della finestra.
            let handle = app.handle().clone();
            let _ = ctrlc::set_handler(move || handle.exit(0));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("avvio di Tauri");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            let state = handle.state::<AppState>();
            if let Some(server) = state.server.lock().unwrap().take() {
                server.shutdown();
            }
            let _ = state.engine.shutdown();
        }
    });
}
