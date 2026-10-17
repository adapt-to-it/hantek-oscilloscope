//! Display dello spettro.

use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use h6022_core::Frame;
use h6022_dsp::{Spectrum, Waveform, Window};

use crate::state::{CH_COLORS, UiState};
use crate::timebase::format_hz;

const DB_MIN: f64 = -120.0;
const DB_MAX: f64 = 20.0;
const DB_STEP: usize = 20;
/// Picchi etichettati per canale.
const PEAKS: usize = 3;

const BACKGROUND: Color32 = Color32::from_gray(6);
const GRID: Color32 = Color32::from_gray(40);
const TEXT: Color32 = Color32::from_gray(190);

/// Spettri calcolati, riusati finché non arriva un frame nuovo o cambia la finestra.
#[derive(Default)]
pub struct SpectrumCache {
    sequence: Option<u64>,
    window: Option<h6022_dsp::Window>,
    spectra: [Option<Spectrum>; 2],
}

impl SpectrumCache {
    /// Spettri dei canali attivi del frame. Ricalcola solo se cambiano frame o finestra.
    pub fn get(&mut self, frame: &Frame, state: &UiState) -> &[Option<Spectrum>; 2] {
        if self.is_stale(frame.sequence, state.window) {
            let enabled = [state.channels[0].enabled, state.channels[1].enabled];
            self.spectra = compute([Some(&frame.ch1), frame.ch2.as_ref()], enabled, state.window);
            self.sequence = Some(frame.sequence);
            self.window = Some(state.window);
        }
        &self.spectra
    }

    fn is_stale(&self, sequence: u64, window: Window) -> bool {
        self.sequence != Some(sequence) || self.window != Some(window)
    }
}

/// Spettro per ogni canale attivo con dati; gli altri restano `None`.
fn compute(waves: [Option<&Waveform>; 2], enabled: [bool; 2], window: Window) -> [Option<Spectrum>; 2] {
    std::array::from_fn(|i| {
        if enabled[i] {
            waves[i].map(|w| h6022_dsp::spectrum::spectrum(w, window))
        } else {
            None
        }
    })
}

/// Disegna lo spettro in dBV dei canali attivi.
pub fn show(ui: &mut Ui, frame: Option<&Frame>, state: &mut UiState, cache: &mut SpectrumCache) {
    let size = ui.available_size();
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = response.rect;
    let plot = Rect::from_min_max(rect.min + Vec2::new(44.0, 8.0), rect.max - Vec2::new(12.0, 22.0));
    painter.rect_filled(rect, 0.0, BACKGROUND);
    if plot.width() <= 1.0 || plot.height() <= 1.0 {
        return;
    }
    draw_db_grid(&painter, plot);

    let Some(frame) = frame else {
        painter.text(plot.center(), Align2::CENTER_CENTER, "In attesa di dati", FontId::proportional(16.0), TEXT);
        return;
    };

    let spectra = cache.get(frame, state);
    let fmax = frame.ch1.sample_rate / 2.0;
    let bin_hz = spectra.iter().flatten().map(|s| s.bin_hz).next().unwrap_or(0.0);
    let log_lo = (state.spectrum_log_freq && bin_hz > 0.0 && fmax > bin_hz).then_some(bin_hz);
    draw_freq_grid(&painter, plot, fmax, log_lo);

    for (i, spec) in spectra.iter().enumerate() {
        let Some(s) = spec else {
            continue;
        };
        if !state.channels[i].enabled {
            continue;
        }
        let color = CH_COLORS[i];
        let dbv = s.dbv();
        draw_trace(&painter, plot, s.bin_hz, &dbv, fmax, log_lo, color);
        for p in s.peaks(PEAKS) {
            let x = freq_to_x(p.frequency, plot, fmax, log_lo);
            let db = vrms_to_dbv(p.vrms);
            let y = db_to_y(db, plot);
            let label = format!("{} {:.1} dBV", format_hz(p.frequency), db);
            painter.text(Pos2::new(x, y - 4.0), Align2::CENTER_BOTTOM, label, FontId::monospace(10.0), color);
        }
    }

    if let Some(p) = response.hover_pos() {
        if plot.contains(p) && bin_hz > 0.0 {
            let freq = x_to_freq(p.x, plot, fmax, log_lo);
            let bin = (freq / bin_hz).round() as usize;
            let reading = spectra.iter().enumerate().find_map(|(i, s)| {
                let s = s.as_ref()?;
                if !state.channels[i].enabled {
                    return None;
                }
                s.vrms.get(bin).copied()
            });
            painter.vline(p.x, plot.y_range(), Stroke::new(1.0, GRID));
            if let Some(v) = reading {
                let text = format!("{} {:.1} dBV", format_hz(freq), vrms_to_dbv(v));
                painter.text(p + Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM, text, FontId::monospace(11.0), TEXT);
            }
        }
    }
}

fn vrms_to_dbv(v: f32) -> f64 {
    20.0 * f64::from(v.max(1e-10)).log10()
}

fn db_to_y(db: f64, plot: Rect) -> f32 {
    let t = (db.clamp(DB_MIN, DB_MAX) - DB_MIN) / (DB_MAX - DB_MIN);
    plot.bottom() - t as f32 * plot.height()
}

/// Ascissa di una frequenza: lineare da 0 a `fmax`, o logaritmica da `log_lo` a `fmax`.
fn freq_to_x(f: f64, plot: Rect, fmax: f64, log_lo: Option<f64>) -> f32 {
    let frac = match log_lo {
        Some(lo) => (f.max(lo) / lo).ln() / (fmax / lo).ln(),
        None => f / fmax,
    };
    plot.left() + frac as f32 * plot.width()
}

fn x_to_freq(x: f32, plot: Rect, fmax: f64, log_lo: Option<f64>) -> f64 {
    let frac = f64::from((x - plot.left()) / plot.width());
    match log_lo {
        Some(lo) => lo * (fmax / lo).powf(frac),
        None => fmax * frac,
    }
}

fn draw_db_grid(painter: &Painter, plot: Rect) {
    let stroke = Stroke::new(1.0, GRID);
    for db in (DB_MIN as i32..=DB_MAX as i32).step_by(DB_STEP) {
        let y = db_to_y(f64::from(db), plot);
        painter.hline(plot.x_range(), y, stroke);
        painter.text(
            Pos2::new(plot.left() - 4.0, y),
            Align2::RIGHT_CENTER,
            format!("{db} dBV"),
            FontId::monospace(10.0),
            TEXT,
        );
    }
}

/// Griglia frequenze: decadi in scala logaritmica, decimi di `fmax` in scala lineare.
fn draw_freq_grid(painter: &Painter, plot: Rect, fmax: f64, log_lo: Option<f64>) {
    let stroke = Stroke::new(1.0, GRID);
    for f in freq_ticks(fmax, log_lo) {
        let x = freq_to_x(f, plot, fmax, log_lo);
        painter.vline(x, plot.y_range(), stroke);
        painter.text(
            Pos2::new(x, plot.bottom() + 3.0),
            Align2::CENTER_TOP,
            format_hz(f),
            FontId::monospace(10.0),
            TEXT,
        );
    }
}

fn freq_ticks(fmax: f64, log_lo: Option<f64>) -> Vec<f64> {
    match log_lo {
        Some(lo) => {
            let first = lo.log10().ceil() as i32;
            (first..).map(|k| 10f64.powi(k)).take_while(|&f| f <= fmax).collect()
        }
        None => (0..=10).map(|k| fmax * f64::from(k) / 10.0).collect(),
    }
}

/// Polilinea dello spettro, con il massimo per colonna di pixel.
fn draw_trace(painter: &Painter, plot: Rect, bin_hz: f64, dbv: &[f32], fmax: f64, log_lo: Option<f64>, color: Color32) {
    let columns = plot.width() as usize;
    let mut top = vec![f32::NEG_INFINITY; columns];
    for (k, &db) in dbv.iter().enumerate() {
        let f = k as f64 * bin_hz;
        if f > fmax {
            break;
        }
        let x = freq_to_x(f, plot, fmax, log_lo);
        let col = ((x - plot.left()).max(0.0) as usize).min(columns - 1);
        top[col] = top[col].max(db);
    }
    let points: Vec<Pos2> = top
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .map(|(c, &v)| Pos2::new(plot.left() + c as f32 + 0.5, db_to_y(f64::from(v), plot)))
        .collect();
    if points.len() >= 2 {
        painter.line(points, Stroke::new(1.2, color));
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Pos2, Rect, Vec2};

    use super::*;

    fn wave(samples: Vec<f32>) -> Waveform {
        Waveform { samples, sample_rate: 1000.0, t0: 0.0 }
    }

    fn sine() -> Vec<f32> {
        (0..256).map(|i| (2.0 * std::f64::consts::PI * 10.0 * i as f64 / 256.0).sin() as f32).collect()
    }

    #[test]
    fn new_cache_is_stale_until_a_frame_and_window_are_recorded() {
        let mut cache = SpectrumCache::default();
        assert!(cache.is_stale(0, Window::FlatTop));
        cache.sequence = Some(3);
        cache.window = Some(Window::FlatTop);
        assert!(!cache.is_stale(3, Window::FlatTop));
    }

    #[test]
    fn cache_goes_stale_on_new_frame_or_new_window() {
        let mut cache = SpectrumCache::default();
        cache.sequence = Some(3);
        cache.window = Some(Window::FlatTop);
        assert!(cache.is_stale(4, Window::FlatTop));
        assert!(cache.is_stale(3, Window::Hann));
    }

    #[test]
    fn compute_skips_disabled_and_missing_channels() {
        let w = wave(sine());
        let out = compute([Some(&w), Some(&w)], [false, true], Window::Hann);
        assert!(out[0].is_none());
        assert!(out[1].is_some());

        let out = compute([Some(&w), None], [true, true], Window::Hann);
        assert!(out[0].is_some());
        assert!(out[1].is_none());
    }

    #[test]
    fn compute_uses_the_requested_window() {
        let w = wave(sine());
        let a = compute([Some(&w), None], [true, false], Window::Hann)[0].clone().unwrap();
        let b = compute([Some(&w), None], [true, false], Window::Rectangular)[0].clone().unwrap();
        assert_ne!(a.vrms, b.vrms);
    }

    #[test]
    fn frequency_mapping_is_inverse_in_both_scales() {
        let plot = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(500.0, 300.0));
        for log_lo in [None, Some(2.0)] {
            for f in [10.0, 100.0, 400.0] {
                let back = x_to_freq(freq_to_x(f, plot, 500.0, log_lo), plot, 500.0, log_lo);
                assert!((back - f).abs() < 1e-2, "log={log_lo:?} f={f} back={back}");
            }
        }
    }
}
