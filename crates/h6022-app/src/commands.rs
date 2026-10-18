//! Comandi invocati dal frontend.

use std::sync::atomic::Ordering;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use h6022_core::{RunState, TriggerMode};
use h6022_view::acquisition::AcquisitionSettings;
use h6022_view::analysis::{SpectrumView, WindowName, XyView, spectrum_view, xy_view};
use h6022_view::measures::{MeasureSnapshot, snapshot};
use h6022_view::settings::ViewSettings;
use h6022_view::trace::encode_display;

use h6022_server::api::{RecordParams, RecordSummaryView, RecordingView};
use h6022_server::api_decode::{ExportParams, ExportView};
use h6022_server::decode_tools::export_view;
use h6022_server::tools::{record_summary_view, recording_view, start_recorder};

use crate::state::AppState;

/// Intervallo minimo tra due frame inviati al display (circa 60 al secondo).
pub const DISPLAY_INTERVAL_MS: u64 = 16;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// "hardware" o "simulator".
    pub source: &'static str,
    /// "running", "armed" o "stopped".
    pub run_state: &'static str,
    /// "auto", "normal", "single" o "roll".
    pub trigger_mode: &'static str,
    pub frames: u64,
    pub sample_rate_hz: f64,
    pub sample_rate_measured_hz: f64,
    pub record_length: usize,
    pub server: String,
}

#[tauri::command]
pub fn status(state: State<'_, AppState>) -> Status {
    let config = state.engine.config();
    let stats = state.engine.stats();
    Status {
        source: state.source,
        run_state: match state.engine.state() {
            RunState::Running => "running",
            RunState::Armed => "armed",
            RunState::Stopped => "stopped",
        },
        trigger_mode: match config.mode {
            TriggerMode::Auto => "auto",
            TriggerMode::Normal => "normal",
            TriggerMode::Single => "single",
            TriggerMode::Roll => "roll",
        },
        frames: stats.frames,
        sample_rate_hz: config.acquisition.sample_rate.hz() as f64,
        sample_rate_measured_hz: stats.sample_rate_measured,
        record_length: config.record_length,
        server: state.server_status.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> ViewSettings {
    *state.view.lock().unwrap()
}

/// Applica la vista alla configurazione del motore. Se il motore la rifiuta, la vista resta quella di prima.
#[tauri::command]
pub fn set_view(state: State<'_, AppState>, view: ViewSettings) -> Result<(), String> {
    let mut config = state.engine.config();
    view.apply(&mut config);
    state.engine.set_config(config).map_err(|e| e.to_string())?;
    *state.view.lock().unwrap() = view;
    Ok(())
}

/// `command`: "run", "stop" o "single".
#[tauri::command]
pub fn run_command(state: State<'_, AppState>, command: String) -> Result<(), String> {
    let mut previous = state.previous_mode.lock().unwrap();
    match command.as_str() {
        "run" => {
            if let Some(mode) = previous.take() {
                let mut config = state.engine.config();
                config.mode = mode;
                state.engine.set_config(config).map_err(|e| e.to_string())?;
            }
            state.engine.run();
        }
        "stop" => state.engine.pause(),
        "single" => {
            let mut config = state.engine.config();
            if config.mode != TriggerMode::Single {
                *previous = Some(config.mode);
            }
            config.mode = TriggerMode::Single;
            state.engine.set_config(config).map_err(|e| e.to_string())?;
            state.engine.run();
        }
        other => return Err(format!("comando sconosciuto: {other}")),
    }
    Ok(())
}

#[tauri::command]
pub fn set_columns(state: State<'_, AppState>, columns: usize) {
    state.columns.store(columns, Ordering::Relaxed);
}

/// Avvia un thread che invia a `channel` ogni frame codificato con `h6022_view::trace::encode_display`.
#[tauri::command]
pub fn subscribe_display(state: State<'_, AppState>, channel: Channel<InvokeResponseBody>) {
    let generation = state.display_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let current = state.display_generation.clone();
    let frames = state.engine.subscribe();
    let view = state.view.clone();
    let columns = state.columns.clone();
    thread::spawn(move || {
        let interval = Duration::from_millis(DISPLAY_INTERVAL_MS);
        let mut last = Instant::now() - interval;
        while current.load(Ordering::SeqCst) == generation {
            // Il timeout serve a ricontrollare la generazione; un canale chiuso vuol dire motore fermo.
            let mut frame = match frames.recv_timeout(Duration::from_millis(200)) {
                Ok(frame) => frame,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            // Solo il frame più recente.
            while let Ok(newer) = frames.try_recv() {
                frame = newer;
            }
            let wait = interval.saturating_sub(last.elapsed());
            if !wait.is_zero() {
                thread::sleep(wait);
            }
            last = Instant::now();
            let settings = *view.lock().unwrap();
            let enabled = [settings.channels[0].enabled, settings.channels[1].enabled];
            let bytes = encode_display(&frame, enabled, &settings.viewport(columns.load(Ordering::Relaxed)));
            if channel.send(InvokeResponseBody::Raw(bytes)).is_err() {
                break;
            }
        }
    });
}


#[tauri::command]
pub fn get_trigger_level(state: State<'_, AppState>) -> f32 {
    state.engine.config().trigger.level
}

#[tauri::command]
pub fn set_trigger_level(state: State<'_, AppState>, volts: f32) -> Result<(), String> {
    let mut config = state.engine.config();
    config.trigger.level = volts;
    state.engine.set_config(config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_acquisition(state: State<'_, AppState>) -> AcquisitionSettings {
    AcquisitionSettings::from_config(&state.engine.config())
}

/// Applica trigger, sonde, accoppiamento e pretrigger. Se il motore li rifiuta, resta la configurazione di prima.
#[tauri::command]
pub fn set_acquisition(state: State<'_, AppState>, settings: AcquisitionSettings) -> Result<(), String> {
    let mut config = state.engine.config();
    settings.apply(&mut config)?;
    // La sonda cambia la portata necessaria: la vista la ricalcola.
    state.view.lock().unwrap().apply(&mut config);
    state.engine.set_config(config).map_err(|e| e.to_string())
}

/// Misure del frame più recente; `None` se non è ancora arrivato un frame.
#[tauri::command]
pub fn measurements(state: State<'_, AppState>) -> Option<MeasureSnapshot> {
    let frame = state.engine.latest_frame()?;
    let view = *state.view.lock().unwrap();
    Some(snapshot(&frame, [view.channels[0].enabled, view.channels[1].enabled]))
}

/// Bin massimi dello spettro inviati al frontend.
pub const SPECTRUM_BINS: usize = 2048;
/// Punti massimi del grafico XY.
pub const XY_POINTS: usize = 4096;

#[tauri::command]
pub fn spectrum(state: State<'_, AppState>, window: WindowName) -> Option<SpectrumView> {
    let frame = state.engine.latest_frame()?;
    let view = *state.view.lock().unwrap();
    let enabled = [view.channels[0].enabled, view.channels[1].enabled];
    Some(spectrum_view(&frame, enabled, window, SPECTRUM_BINS))
}

#[tauri::command]
pub fn xy(state: State<'_, AppState>) -> Option<XyView> {
    let frame = state.engine.latest_frame()?;
    Some(xy_view(&frame, XY_POINTS))
}

/// Vista iniziale scelta con `H6022_MODE` (`time`, `spectrum`, `xy`); serve alle catture automatiche.
#[tauri::command]
pub fn initial_mode() -> Option<String> {
    std::env::var("H6022_MODE").ok().filter(|m| matches!(m.as_str(), "time" | "spectrum" | "xy"))
}

/// Decodifica il frame più recente con i parametri dello strumento MCP `decode`.
/// `None` se non è ancora arrivato un frame; errore se i parametri non sono validi.
#[tauri::command]
pub fn decode(
    state: State<'_, AppState>,
    params: h6022_server::api_decode::DecodeParams,
) -> Result<Option<h6022_server::api_decode::DecodeView>, String> {
    let Some(frame) = state.engine.latest_frame() else { return Ok(None) };
    h6022_server::decode_tools::decode_view(&frame, params).map(Some).map_err(|e| e.message.to_string())
}

/// Avvia una registrazione con i parametri dello strumento MCP `start_recording`.
/// Una registrazione già finita per limite viene chiusa prima; una ancora attiva dà errore.
#[tauri::command]
pub fn start_recording(state: State<'_, AppState>, params: RecordParams) -> Result<RecordingView, String> {
    let mut slot = state.recorder.lock().unwrap();
    if let Some(old) = slot.take() {
        if old.status().running {
            *slot = Some(old);
            return Err("registrazione già attiva".into());
        }
        let _ = old.stop();
    }
    let recorder = start_recorder(&state.engine, params).map_err(|e| e.message.to_string())?;
    let view = recording_view(recorder.status());
    *slot = Some(recorder);
    Ok(view)
}

/// Ferma la registrazione e restituisce il riepilogo.
#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<RecordSummaryView, String> {
    let recorder = state.recorder.lock().unwrap().take().ok_or("nessuna registrazione attiva")?;
    recorder.stop().map(record_summary_view).map_err(|e| e.to_string())
}

/// Stato della registrazione; `None` se non ne è stata avviata una.
#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> Option<RecordingView> {
    state.recorder.lock().unwrap().as_ref().map(|r| recording_view(r.status()))
}

/// Esporta il frame più recente in vcd o sr con i parametri dello strumento MCP `export_logic`.
#[tauri::command]
pub fn export_logic(state: State<'_, AppState>, params: ExportParams) -> Result<ExportView, String> {
    let frame = state.engine.latest_frame().ok_or("nessun frame da esportare")?;
    export_view(&frame, params).map_err(|e| e.message.to_string())
}
