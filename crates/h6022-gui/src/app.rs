//! Applicazione: collega motore, stato e pannelli.

use std::collections::VecDeque;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Key, Layout, RichText};
use h6022_core::{Engine, EngineConfig, EngineHandle, Frame, RunState, SampleSource, Slope, TriggerConfig, TriggerKind, TriggerMode};
use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

use crate::controls::{self, RunCommand};
use crate::decode_panel::{self, DecodeState};
use crate::measure_panel::{self, MeasureCache};
use crate::scope_view;
use crate::spectrum_view::{self, SpectrumCache};
use crate::state::{UiState, View};

/// Finestra su cui si calcolano i frame al secondo.
const FPS_WINDOW: Duration = Duration::from_secs(2);
/// Durata di visualizzazione di un errore di configurazione.
const ERROR_TTL: Duration = Duration::from_secs(5);
/// Attesa prima dell'immagine automatica, per avere già frame a schermo.
const SCREENSHOT_DELAY: Duration = Duration::from_secs(3);

pub struct App {
    engine: EngineHandle,
    config: EngineConfig,
    state: UiState,
    frame: Option<Arc<Frame>>,
    measures: MeasureCache,
    spectra: SpectrumCache,
    /// Decoder dei protocolli: scelta e annotazioni sotto il display.
    decode: DecodeState,
    /// Contatori di frame con l'istante di lettura, per i frame al secondo.
    fps_samples: VecDeque<(Instant, u64)>,
    /// Ultimo errore di configurazione, con l'istante in cui è apparso.
    error: Option<(Instant, String)>,
    /// Modalità di trigger prima dell'ultimo `Single`, ripristinata da `Run`.
    previous_mode: Option<TriggerMode>,
    /// Immagine automatica chiesta con `H6022_SCREENSHOT`: percorso e istante di scatto.
    auto_screenshot: Option<(Instant, String)>,
    /// Server MCP e WebSocket nello stesso processo, se attivo.
    server: Option<h6022_server::ServerHandle>,
    /// Indirizzo del server o motivo per cui non è partito.
    server_status: String,
}

impl App {
    /// Avvia il motore e un thread che chiede un ridisegno a ogni frame nuovo.
    pub fn new(ctx: egui::Context, source: Box<dyn SampleSource>) -> App {
        let state = UiState::default();
        let mut config = EngineConfig {
            acquisition: AcquisitionConfig {
                sample_rate: SampleRate::nearest(1e6),
                mode: ChannelMode::Dual,
                range: [VoltageRange::V5; 2],
                coupling: [Coupling::Dc; 2],
            },
            trigger: TriggerConfig {
                source: Channel::Ch1,
                kind: TriggerKind::Edge { slope: Slope::Rising },
                level: 1.5,
                hysteresis: 0.1,
                holdoff: 0.0,
            },
            mode: TriggerMode::Auto,
            record_length: 1000,
            pretrigger: 0.5,
            probe: [1.0, 1.0],
        };
        controls::sync_config(&state, &mut config);

        let engine = Engine::start(source, config).unwrap_or_else(|e| panic!("avvio del motore fallito: {e}"));
        let frames = engine.subscribe();
        thread::spawn(move || {
            for _ in frames {
                ctx.request_repaint();
            }
        });

        App {
            engine,
            config,
            state,
            frame: None,
            measures: MeasureCache::default(),
            spectra: SpectrumCache::default(),
            decode: DecodeState::default(),
            fps_samples: VecDeque::new(),
            error: None,
            previous_mode: None,
            auto_screenshot: std::env::var("H6022_SCREENSHOT")
                .ok()
                .map(|path| (Instant::now() + SCREENSHOT_DELAY, path)),
            server: None,
            server_status: "MCP spento".to_string(),
        }
    }

    /// Prepara una vista iniziale da `H6022_SCENE`, per catturare le schermate.
    /// Parole separate da virgola: `time`, `spectrum`, `xy`, `help`, `persistence`,
    /// `cursors`, `decode=<protocollo>` (per esempio `decode=pwm`).
    pub fn apply_scene(&mut self, scene: &str) {
        for word in scene.split(',').map(str::trim) {
            match word {
                "time" => self.state.view = View::Time,
                "spectrum" => self.state.view = View::Spectrum,
                "xy" => self.state.view = View::Xy,
                "help" => self.state.help = true,
                "persistence" => self.state.persistence = true,
                "cursors" => self.state.cursors.mode = crate::state::CursorMode::Both,
                _ => {
                    let Some(name) = word.strip_prefix("decode=") else {
                        eprintln!("H6022_SCENE: parola sconosciuta {word:?}");
                        continue;
                    };
                    match decode_panel::protocol_templates().iter().position(|p| p.name() == name) {
                        Some(index) => {
                            self.decode.enabled = true;
                            self.decode.selected = index;
                        }
                        None => eprintln!("H6022_SCENE: protocollo sconosciuto {name:?}"),
                    }
                }
            }
        }
    }

    /// Avvia il server MCP e WebSocket sul motore dell'app. Un errore resta visibile nella barra.
    pub fn start_server(&mut self, source: &'static str, port: u16) {
        let config = h6022_server::ServerConfig {
            addr: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        };
        match h6022_server::spawn(self.engine.clone(), source, config) {
            Ok(server) => {
                self.server_status = format!("MCP http://{}/mcp", server.addr());
                self.server = Some(server);
            }
            Err(e) => self.server_status = format!("MCP non avviato: {e}"),
        }
    }

    /// Disegna l'intera interfaccia. Separato da `eframe::App` per i test.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.frame = self.engine.latest_frame();
        let run_state = self.engine.state();
        let stats = self.engine.stats();
        let now = Instant::now();
        let fps = update_fps(&mut self.fps_samples, now, stats.frames);
        if self.error.as_ref().is_some_and(|(t, _)| now.duration_since(*t) >= ERROR_TTL) {
            self.error = None;
        }

        let ctx = ui.ctx().clone();
        let before = self.config;
        if let Some((at, path)) = self.auto_screenshot.take() {
            if now >= at {
                request_png(&ctx, path);
            } else {
                ctx.request_repaint_after(at - now);
                self.auto_screenshot = Some((at, path));
            }
        }

        // Tasti: ignorati mentre l'utente scrive in un campo di testo.
        let mut key_command = None;
        if !ctx.egui_wants_keyboard_input() {
            let (help, run, single, view, shot) = ctx.input(|i| {
                (
                    i.key_pressed(Key::I),
                    i.key_pressed(Key::Space),
                    i.key_pressed(Key::S),
                    i.key_pressed(Key::F),
                    i.key_pressed(Key::P),
                )
            });
            if shot {
                let name = format!("h6022-{}.png", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()));
                request_png(&ctx, name);
            }
            if help {
                self.state.help = !self.state.help;
            }
            if run {
                key_command = Some(if run_state == RunState::Running { RunCommand::Stop } else { RunCommand::Run });
            }
            if single {
                key_command = Some(RunCommand::Single);
            }
            if view {
                self.state.view = if self.state.view == View::Spectrum { View::Time } else { View::Spectrum };
            }
        }

        egui::Panel::top("status").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Motore: {}", engine_state_name(run_state)));
                ui.label(format!("{fps:.1} frame/s"));
                ui.label(&self.server_status);
                if let Some((_, message)) = &self.error {
                    ui.label(RichText::new(format!("Errore: {message}")).color(Color32::RED));
                }
            });
        });

        let mut command = None;
        egui::Panel::right("controls").default_size(300.0).show_inside(ui, |ui| {
            // Il pannello dei decoder sta in basso: i controlli esistenti occupano il resto.
            egui::Panel::bottom("decode_controls").resizable(true).default_size(260.0).show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    decode_panel::controls(ui, &self.state, &mut self.decode);
                });
            });
            command = controls::show(ui, &mut self.state, &mut self.config, run_state, &stats);
        });

        egui::Panel::bottom("measures").resizable(true).show_inside(ui, |ui| {
            measure_panel::show(ui, self.frame.as_deref(), &self.state, &mut self.measures);
        });

        let mut trigger_level = None;
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.state.help {
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.label("Spiegazioni attive (i)");
                });
            }
            // Con il decoder attivo, sotto il display resta lo spazio della striscia.
            let reserved = if self.decode.enabled { decode_panel::RESERVED_HEIGHT } else { 0.0 };
            let size = egui::vec2(ui.available_width(), (ui.available_height() - reserved).max(0.0));
            let display = ui.allocate_ui(size, |ui| match self.state.view {
                View::Spectrum => {
                    spectrum_view::show(ui, self.frame.as_deref(), &mut self.state, &mut self.spectra);
                    None
                }
                View::Time | View::Xy => {
                    let response = scope_view::show(ui, self.frame.as_deref(), &mut self.state, &self.config.trigger);
                    response.trigger_level
                }
            });
            trigger_level = display.inner;
            if self.decode.enabled {
                decode_panel::strip(ui, display.response.rect, self.frame.as_deref(), &mut self.decode, &self.state);
            }
        });
        if let Some(level) = trigger_level {
            self.config.trigger.level = level;
        }

        if self.config != before {
            self.push_config();
        }

        match key_command.or(command) {
            Some(RunCommand::Run) => {
                if let Some(mode) = self.previous_mode.take() {
                    self.config.mode = mode;
                    self.push_config();
                }
                self.engine.run();
            }
            Some(RunCommand::Stop) => self.engine.pause(),
            Some(RunCommand::Single) => {
                if self.config.mode != TriggerMode::Single {
                    self.previous_mode = Some(self.config.mode);
                }
                self.config.mode = TriggerMode::Single;
                self.push_config();
                self.engine.run();
            }
            None => {}
        }
    }

    /// Invia la configurazione al motore; un errore resta visibile per `ERROR_TTL`.
    fn push_config(&mut self) {
        if let Err(e) = self.engine.set_config(self.config) {
            self.error = Some((Instant::now(), e.to_string()));
        }
    }
}

/// Chiede a egui l'immagine della finestra e la salva in PNG nel percorso dato.
fn request_png(ctx: &egui::Context, path: String) {
    ctx.request_screenshot(move |image| {
        let [w, h] = image.size;
        let result = image::RgbaImage::from_raw(w as u32, h as u32, image.as_raw().to_vec())
            .ok_or_else(|| "dimensioni dell'immagine incoerenti".to_string())
            .and_then(|img| img.save(&path).map_err(|e| e.to_string()));
        match result {
            Ok(()) => eprintln!("immagine salvata in {path}"),
            Err(e) => eprintln!("salvataggio di {path} fallito: {e}"),
        }
    });
    ctx.request_repaint();
}

/// Frame al secondo sugli ultimi `FPS_WINDOW`, dai contatori di frame letti a istanti diversi.
fn update_fps(samples: &mut VecDeque<(Instant, u64)>, now: Instant, frames: u64) -> f64 {
    samples.push_back((now, frames));
    while samples.front().is_some_and(|(t, _)| now.duration_since(*t) > FPS_WINDOW) {
        samples.pop_front();
    }
    match (samples.front(), samples.back()) {
        (Some(&(t0, f0)), Some(&(t1, f1))) if t1 > t0 => {
            f1.saturating_sub(f0) as f64 / t1.duration_since(t0).as_secs_f64()
        }
        _ => 0.0,
    }
}

fn engine_state_name(state: RunState) -> &'static str {
    match state {
        RunState::Running => "in esecuzione",
        RunState::Armed => "in attesa di trigger",
        RunState::Stopped => "fermo",
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    fn on_exit(&mut self) {
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
        let _ = self.engine.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use eframe::egui::RawInput;
    use h6022_core::sim::{Shape, SignalSpec, SimulatedSource};

    use super::*;

    const BLOCK_LEN: usize = 1024;

    fn sine() -> SignalSpec {
        SignalSpec { shape: Shape::Sine, frequency: 1_000.0, amplitude: 1.0, offset: 0.0, phase_deg: 0.0, noise: 0.0 }
    }

    fn draw_frames(ctx: &egui::Context, app: &mut App, n: usize) {
        for _ in 0..n {
            // Senza renderer nessuno applica le texture: il test le scarta.
            ctx.run_ui(RawInput::default(), |ui| app.draw(ui)).textures_delta.clear();
            thread::sleep(Duration::from_millis(20));
        }
        assert!(app.frame.is_some());
    }

    #[test]
    fn draws_every_view_and_help_then_shuts_down() {
        let ctx = egui::Context::default();
        let source = SimulatedSource::new([sine(), sine()], BLOCK_LEN, false);
        let mut app = App::new(ctx.clone(), Box::new(source));

        draw_frames(&ctx, &mut app, 20);

        app.state.view = View::Spectrum;
        draw_frames(&ctx, &mut app, 20);

        app.state.view = View::Xy;
        draw_frames(&ctx, &mut app, 20);

        app.state.view = View::Time;
        app.state.help = true;
        draw_frames(&ctx, &mut app, 20);

        // Decoder attivo su PWM: la striscia e le annotazioni vengono disegnate.
        app.decode.enabled = true;
        app.decode.selected = decode_panel::protocol_templates().iter().position(|p| p.name() == "pwm").expect("pwm");
        draw_frames(&ctx, &mut app, 20);

        app.engine.shutdown().expect("arresto del motore");
    }
}
