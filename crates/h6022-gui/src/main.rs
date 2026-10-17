//! Oscilloscopio Hantek 6022BE: interfaccia grafica.

mod app;
mod controls;
mod help;
mod measure_panel;
mod scope_view;
mod spectrum_view;
mod state;
mod timebase;

use h6022_core::SampleSource;
use h6022_core::hardware::HardwareSource;
use h6022_core::sim::SimulatedSource;

fn main() -> eframe::Result {
    // `--sim` usa il simulatore al posto del dispositivo.
    // `--no-server` non avvia il server MCP; `H6022_PORT` cambia la porta.
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
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Hantek 6022BE")
            .with_inner_size([1280.0, 800.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "h6022-gui",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc.egui_ctx.clone(), source);
            if with_server {
                app.start_server(name, port);
            }
            Ok(Box::new(app))
        }),
    )
}
