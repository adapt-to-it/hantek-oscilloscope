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
    let simulated = std::env::args().any(|a| a == "--sim");
    let source: Box<dyn SampleSource> = if simulated {
        Box::new(SimulatedSource::bench_default())
    } else {
        match HardwareSource::open() {
            Ok(source) => Box::new(source),
            Err(e) => {
                eprintln!("dispositivo non disponibile ({e}): uso il simulatore");
                Box::new(SimulatedSource::bench_default())
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
        Box::new(move |cc| Ok(Box::new(app::App::new(cc.egui_ctx.clone(), source)))),
    )
}
