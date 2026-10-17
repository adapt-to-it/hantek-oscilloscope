//! Display nel dominio del tempo e XY.

use std::collections::VecDeque;

use eframe::egui::{Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use h6022_core::{Frame, TriggerConfig};
use h6022_dsp::Waveform;
use h6022_dsp::resample::{min_max_decimate, sinc_interpolate};
use h6022_usb::Channel;

use crate::state::{CH_COLORS, CursorMode, UiState, View};
use crate::timebase::{DIVISIONS_X, DIVISIONS_Y, format_hz, format_time, format_volts};

/// Frame precedenti conservati per la persistenza.
const PERSISTENCE_FRAMES: usize = 8;
/// Fattore di sovracampionamento quando i campioni sono pochi per pixel.
const UPSAMPLE: usize = 4;
const HISTORY_ID: &str = "scope_persistence";
/// Punti massimi disegnati nella vista XY.
const XY_MAX_POINTS: usize = 50_000;

const BACKGROUND: Color32 = Color32::from_gray(6);
const GRID: Color32 = Color32::from_gray(40);
const AXIS: Color32 = Color32::from_gray(110);
const TEXT: Color32 = Color32::from_gray(190);
const CURSOR: Color32 = Color32::from_gray(220);
const READOUT_BG: Color32 = Color32::from_black_alpha(190);

/// Punto di una traccia: tempo in secondi e tensione in volt.
type TracePoint = (f64, f32);
/// Traccia di un canale.
type Trace = (usize, Vec<TracePoint>);

/// Tracce dei frame precedenti, per la persistenza. Conservato in memoria egui.
#[derive(Clone, Default)]
struct History {
    sequence: Option<u64>,
    frames: VecDeque<Vec<Trace>>,
}

/// Effetti dell'interazione con il display.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScopeResponse {
    /// Nuovo livello di trigger in volt, se l'utente ha trascinato il marcatore.
    pub trigger_level: Option<f32>,
}

/// Disegna il display nel dominio del tempo (o XY se `state.view` è `View::Xy`) nello spazio disponibile.
pub fn show(ui: &mut Ui, frame: Option<&Frame>, state: &mut UiState, trigger: &TriggerConfig) -> ScopeResponse {
    let size = ui.available_size();
    let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0.0, BACKGROUND);
    draw_grid(&painter, rect);
    draw_axes(&painter, rect);

    let Some(frame) = frame else {
        painter.text(rect.center(), Align2::CENTER_CENTER, "In attesa di dati", FontId::proportional(16.0), TEXT);
        return ScopeResponse::default();
    };

    if state.view == View::Xy {
        draw_xy(&painter, rect, frame, state);
        return ScopeResponse::default();
    }

    if state.persistence {
        let current = collect_traces(rect, frame, state);
        let past = update_history(ui, frame.sequence, &current);
        draw_history(&painter, rect, &past, state);
    } else {
        clear_history(ui);
    }

    for (ch, wave) in active_waves(frame, state) {
        let (start, end) = visible_range(wave, state);
        draw_wave(&painter, rect, wave, ch, state, start, end);
    }

    draw_zero_markers(&painter, rect, state);
    let trigger_level = draw_trigger(ui, &painter, rect, state, trigger);
    draw_cursors(ui, &painter, rect, state);
    draw_cursor_readout(&painter, rect, state);
    draw_status(&painter, rect, frame, state);

    ScopeResponse { trigger_level }
}

/// Ascissa sullo schermo del tempo `t` (secondi dal trigger).
pub fn time_to_x(rect: Rect, t: f64, state: &UiState) -> f32 {
    let px_per_div = rect.width() / DIVISIONS_X;
    rect.center().x + ((t - state.horizontal_offset) / state.time_per_div) as f32 * px_per_div
}

pub fn x_to_time(rect: Rect, x: f32, state: &UiState) -> f64 {
    let px_per_div = rect.width() / DIVISIONS_X;
    state.horizontal_offset + f64::from((x - rect.center().x) / px_per_div) * state.time_per_div
}

/// Ordinata sullo schermo della tensione `v` del canale `ch` (0 o 1).
pub fn volts_to_y(rect: Rect, v: f32, ch: usize, state: &UiState) -> f32 {
    let c = state.channels[ch];
    let px_per_div = rect.height() / DIVISIONS_Y;
    rect.center().y - c.offset_div * px_per_div - v / c.volts_per_div * px_per_div
}

pub fn y_to_volts(rect: Rect, y: f32, ch: usize, state: &UiState) -> f32 {
    let c = state.channels[ch];
    let px_per_div = rect.height() / DIVISIONS_Y;
    (rect.center().y - c.offset_div * px_per_div - y) / px_per_div * c.volts_per_div
}

/// Indice del canale (0 o 1) di una sorgente di trigger o di un cursore.
fn channel_index(ch: Channel) -> usize {
    match ch {
        Channel::Ch1 => 0,
        _ => 1,
    }
}

/// Canali attivi con dati, con il loro indice.
fn active_waves<'a>(frame: &'a Frame, state: &UiState) -> Vec<(usize, &'a Waveform)> {
    [Some(&frame.ch1), frame.ch2.as_ref()]
        .into_iter()
        .enumerate()
        .filter_map(|(c, w)| w.filter(|_| state.channels[c].enabled).map(|w| (c, w)))
        .collect()
}

fn sample_time(wave: &Waveform, i: usize) -> f64 {
    wave.t0 + i as f64 / wave.sample_rate
}

/// Indici dei campioni visibili: l'intervallo di 10 divisioni centrato su `horizontal_offset`.
fn visible_range(wave: &Waveform, state: &UiState) -> (usize, usize) {
    let half = f64::from(DIVISIONS_X) / 2.0 * state.time_per_div;
    let first = ((state.horizontal_offset - half - wave.t0) * wave.sample_rate).floor().max(0.0) as usize;
    let last = ((state.horizontal_offset + half - wave.t0) * wave.sample_rate).ceil().max(0.0) as usize + 1;
    let end = last.min(wave.samples.len());
    (first.min(end), end)
}

fn trace_points(wave: &Waveform, start: usize, end: usize, stride: usize) -> Vec<TracePoint> {
    (start..end)
        .step_by(stride.max(1))
        .map(|i| (sample_time(wave, i), wave.samples[i]))
        .collect()
}

/// Disegna un canale nel suo intervallo visibile, con il metodo adatto alla densità dei campioni.
fn draw_wave(painter: &Painter, rect: Rect, wave: &Waveform, ch: usize, state: &UiState, start: usize, end: usize) {
    let n = end - start;
    if n == 0 {
        return;
    }
    let columns = rect.width().max(1.0) as usize;
    let samples_per_px = n as f64 / columns as f64;
    let stroke = Stroke::new(1.2, CH_COLORS[ch]);
    let samples = &wave.samples[start..end];

    if samples_per_px > 2.0 {
        let buckets = min_max_decimate(samples, columns);
        if buckets.is_empty() {
            return;
        }
        let step = rect.width() / buckets.len() as f32;
        let mut points = Vec::with_capacity(buckets.len() * 2);
        for (col, &(a, b)) in buckets.iter().enumerate() {
            let x = rect.left() + (col as f32 + 0.5) * step;
            let (lo, hi) = (a.min(b), a.max(b));
            points.push(Pos2::new(x, volts_to_y(rect, hi, ch, state)));
            points.push(Pos2::new(x, volts_to_y(rect, lo, ch, state)));
        }
        painter.line(points, stroke);
    } else if samples_per_px < 0.25 {
        let up = sinc_interpolate(samples, UPSAMPLE);
        let t_start = sample_time(wave, start);
        let step = 1.0 / (UPSAMPLE as f64 * wave.sample_rate);
        let points: Vec<Pos2> = up
            .iter()
            .enumerate()
            .map(|(k, &v)| {
                let t = t_start + k as f64 * step;
                Pos2::new(time_to_x(rect, t, state), volts_to_y(rect, v, ch, state))
            })
            .collect();
        painter.line(points, stroke);
    } else {
        let points: Vec<Pos2> = (start..end)
            .map(|i| {
                let x = time_to_x(rect, sample_time(wave, i), state);
                Pos2::new(x, volts_to_y(rect, wave.samples[i], ch, state))
            })
            .collect();
        painter.line(points, stroke);
    }
}

/// Tracce del frame corrente ridotte a circa un punto per pixel, per la persistenza.
fn collect_traces(rect: Rect, frame: &Frame, state: &UiState) -> Vec<Trace> {
    let columns = rect.width().max(1.0) as usize;
    active_waves(frame, state)
        .into_iter()
        .map(|(ch, wave)| {
            let (start, end) = visible_range(wave, state);
            let stride = ((end - start) / columns).max(1);
            (ch, trace_points(wave, start, end, stride))
        })
        .collect()
}

/// Registra il frame se è nuovo e restituisce i frame conservati, dal più vecchio.
fn update_history(ui: &mut Ui, sequence: u64, current: &[Trace]) -> Vec<Vec<Trace>> {
    ui.data_mut(|d| {
        let history = d.get_temp_mut_or_default::<History>(Id::new(HISTORY_ID));
        if history.sequence != Some(sequence) {
            history.sequence = Some(sequence);
            history.frames.push_back(current.to_vec());
            while history.frames.len() > PERSISTENCE_FRAMES {
                history.frames.pop_front();
            }
        }
        history.frames.iter().cloned().collect()
    })
}

fn clear_history(ui: &mut Ui) {
    ui.data_mut(|d| *d.get_temp_mut_or_default::<History>(Id::new(HISTORY_ID)) = History::default());
}

/// Disegna i frame precedenti con alfa crescente verso il più recente (quello corrente escluso).
fn draw_history(painter: &Painter, rect: Rect, past: &[Vec<Trace>], state: &UiState) {
    let n = past.len();
    for (k, traces) in past.iter().take(n.saturating_sub(1)).enumerate() {
        let alpha = ((k + 1) as f32 / n as f32 * 96.0) as u8;
        for (ch, points) in traces {
            if points.len() < 2 {
                continue;
            }
            let c = CH_COLORS[*ch];
            let line: Vec<Pos2> = points
                .iter()
                .map(|&(t, v)| Pos2::new(time_to_x(rect, t, state), volts_to_y(rect, v, *ch, state)))
                .collect();
            let color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
            painter.line(line, Stroke::new(1.0, color));
        }
    }
}

/// Marcatore del trigger sul bordo destro, triangolo in t = 0 sul bordo superiore.
/// Restituisce il nuovo livello in volt mentre il marcatore viene trascinato.
fn draw_trigger(ui: &mut Ui, painter: &Painter, rect: Rect, state: &UiState, trigger: &TriggerConfig) -> Option<f32> {
    let src = channel_index(trigger.source);
    let color = CH_COLORS[src];
    let y = volts_to_y(rect, trigger.level, src, state);
    let right = rect.right();
    painter.add(Shape::convex_polygon(
        vec![Pos2::new(right, y), Pos2::new(right - 10.0, y - 6.0), Pos2::new(right - 10.0, y + 6.0)],
        color,
        Stroke::NONE,
    ));

    let x0 = time_to_x(rect, 0.0, state);
    if (rect.left()..=rect.right()).contains(&x0) {
        painter.add(Shape::convex_polygon(
            vec![Pos2::new(x0, rect.top()), Pos2::new(x0 - 6.0, rect.top() + 10.0), Pos2::new(x0 + 6.0, rect.top() + 10.0)],
            color,
            Stroke::NONE,
        ));
    }

    let hit = Rect::from_center_size(Pos2::new(right - 5.0, y), Vec2::splat(18.0));
    let response = ui.interact(hit, ui.id().with("trigger_marker"), Sense::drag());
    if response.dragged() {
        response.interact_pointer_pos().map(|p| y_to_volts(rect, p.y, src, state))
    } else {
        None
    }
}

/// Marcatore dello zero di ogni canale attivo sul bordo sinistro.
fn draw_zero_markers(painter: &Painter, rect: Rect, state: &UiState) {
    for (ch, c) in state.channels.iter().enumerate() {
        if !c.enabled {
            continue;
        }
        let y = volts_to_y(rect, 0.0, ch, state);
        if (rect.top()..=rect.bottom()).contains(&y) {
            let left = rect.left();
            painter.add(Shape::convex_polygon(
                vec![Pos2::new(left, y), Pos2::new(left + 10.0, y - 6.0), Pos2::new(left + 10.0, y + 6.0)],
                CH_COLORS[ch],
                Stroke::NONE,
            ));
        }
    }
}

/// Cursori tratteggiati, trascinabili secondo `state.cursors.mode`.
fn draw_cursors(ui: &mut Ui, painter: &Painter, rect: Rect, state: &mut UiState) {
    let mode = state.cursors.mode;
    if mode == CursorMode::Off {
        return;
    }
    if matches!(mode, CursorMode::Time | CursorMode::Both) {
        for i in 0..2 {
            let x = time_to_x(rect, state.cursors.t[i], state);
            dashed_line(painter, Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom()));
            let hit = Rect::from_min_max(Pos2::new(x - 5.0, rect.top()), Pos2::new(x + 5.0, rect.bottom()));
            let response = ui.interact(hit, ui.id().with(("cursor_t", i)), Sense::drag());
            if response.dragged() {
                if let Some(p) = response.interact_pointer_pos() {
                    let t = x_to_time(rect, p.x, state);
                    state.cursors.t[i] = t;
                }
            }
        }
    }
    if matches!(mode, CursorMode::Voltage | CursorMode::Both) {
        let ch = channel_index(state.cursors.channel);
        for i in 0..2 {
            let y = volts_to_y(rect, state.cursors.v[i], ch, state);
            dashed_line(painter, Pos2::new(rect.left(), y), Pos2::new(rect.right(), y));
            let hit = Rect::from_min_max(Pos2::new(rect.left(), y - 5.0), Pos2::new(rect.right(), y + 5.0));
            let response = ui.interact(hit, ui.id().with(("cursor_v", i)), Sense::drag());
            if response.dragged() {
                if let Some(p) = response.interact_pointer_pos() {
                    let v = y_to_volts(rect, p.y, ch, state);
                    state.cursors.v[i] = v;
                }
            }
        }
    }
}

fn dashed_line(painter: &Painter, a: Pos2, b: Pos2) {
    painter.extend(Shape::dashed_line(&[a, b], Stroke::new(1.0, CURSOR), 6.0, 4.0));
}

/// Riquadro in alto a sinistra con i valori dei cursori.
fn draw_cursor_readout(painter: &Painter, rect: Rect, state: &UiState) {
    let c = state.cursors;
    if c.mode == CursorMode::Off {
        return;
    }
    let dt = c.t[1] - c.t[0];
    let dv = c.v[1] - c.v[0];
    let inv_dt = if dt != 0.0 { format_hz(1.0 / dt.abs()) } else { "-".to_string() };
    let lines = [
        format!("t1 {}", format_time(c.t[0])),
        format!("t2 {}", format_time(c.t[1])),
        format!("Δt {}", format_time(dt)),
        format!("1/Δt {inv_dt}"),
        format!("V1 {}", format_volts(c.v[0])),
        format!("V2 {}", format_volts(c.v[1])),
        format!("ΔV {}", format_volts(dv)),
    ];
    let origin = rect.left_top() + Vec2::new(6.0, 6.0);
    let line_h = 15.0;
    let box_rect = Rect::from_min_size(origin, Vec2::new(150.0, line_h * lines.len() as f32 + 8.0));
    painter.rect_filled(box_rect, 3.0, READOUT_BG);
    for (i, text) in lines.into_iter().enumerate() {
        let pos = origin + Vec2::new(6.0, 4.0 + i as f32 * line_h);
        painter.text(pos, Align2::LEFT_TOP, text, FontId::monospace(11.0), TEXT);
    }
}

/// Testo in basso: V/div, tempo/div, frequenza di campionamento, stato del trigger.
fn draw_status(painter: &Painter, rect: Rect, frame: &Frame, state: &UiState) {
    let mut parts: Vec<(String, Color32)> = Vec::new();
    for (c, ch) in state.channels.iter().enumerate() {
        if ch.enabled {
            parts.push((format!("CH{} {}/div", c + 1, format_volts(ch.volts_per_div)), CH_COLORS[c]));
        }
    }
    parts.push((format!("{}/div", format_time(state.time_per_div)), TEXT));
    parts.push((format!("fs {}", format_hz(frame.ch1.sample_rate)), TEXT));
    let mode = if frame.triggered { "TRIG'D" } else { "AUTO" };
    parts.push((mode.to_string(), TEXT));

    let mut pos = Pos2::new(rect.left() + 6.0, rect.bottom() - 6.0);
    for (text, color) in parts {
        let r = painter.text(pos, Align2::LEFT_BOTTOM, text, FontId::monospace(11.0), color);
        pos.x = r.right() + 14.0;
    }
}

fn draw_grid(painter: &Painter, rect: Rect) {
    let stroke = Stroke::new(1.0, GRID);
    let px_x = rect.width() / DIVISIONS_X;
    let px_y = rect.height() / DIVISIONS_Y;
    for i in 0..=DIVISIONS_X as usize {
        painter.vline(rect.left() + i as f32 * px_x, rect.y_range(), stroke);
    }
    for j in 0..=DIVISIONS_Y as usize {
        painter.hline(rect.x_range(), rect.top() + j as f32 * px_y, stroke);
    }
}

/// Assi centrali con tacche a 1/5 di divisione.
fn draw_axes(painter: &Painter, rect: Rect) {
    let center = rect.center();
    let stroke = Stroke::new(1.0, AXIS);
    painter.vline(center.x, rect.y_range(), stroke);
    painter.hline(rect.x_range(), center.y, stroke);

    let sub_x = rect.width() / DIVISIONS_X / 5.0;
    let sub_y = rect.height() / DIVISIONS_Y / 5.0;
    let half_x = (DIVISIONS_X * 2.5) as i32;
    let half_y = (DIVISIONS_Y * 2.5) as i32;
    for k in -half_x..=half_x {
        let x = center.x + k as f32 * sub_x;
        painter.line_segment([Pos2::new(x, center.y - 3.0), Pos2::new(x, center.y + 3.0)], stroke);
    }
    for k in -half_y..=half_y {
        let y = center.y + k as f32 * sub_y;
        painter.line_segment([Pos2::new(center.x - 3.0, y), Pos2::new(center.x + 3.0, y)], stroke);
    }
}

/// Vista XY: CH1 su X, CH2 su Y, ciascuno con la propria scala in V/div.
fn draw_xy(painter: &Painter, rect: Rect, frame: &Frame, state: &UiState) {
    let Some(ch2) = frame.ch2.as_ref() else {
        return;
    };
    if !(state.channels[0].enabled && state.channels[1].enabled) {
        return;
    }
    let px_x = rect.width() / DIVISIONS_X;
    let x_ch = state.channels[0];
    let n = frame.ch1.samples.len().min(ch2.samples.len());
    let stride = (n / XY_MAX_POINTS).max(1);
    let base = CH_COLORS[0];
    let color = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), (0.3 * 255.0) as u8);
    for i in (0..n).step_by(stride) {
        let x = rect.center().x + (frame.ch1.samples[i] / x_ch.volts_per_div + x_ch.offset_div) * px_x;
        let y = volts_to_y(rect, ch2.samples[i], 1, state);
        painter.circle_filled(Pos2::new(x, y), 1.0, color);
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Pos2, Rect, Vec2};

    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1000.0, 800.0))
    }

    #[test]
    fn centre_x_is_horizontal_offset() {
        let mut state = UiState::default();
        state.horizontal_offset = 2e-4;
        assert!((time_to_x(rect(), 2e-4, &state) - 500.0).abs() < 1e-3);
    }

    #[test]
    fn ten_divisions_cover_the_width() {
        let state = UiState::default();
        let x = time_to_x(rect(), 5.0 * state.time_per_div, &state);
        assert!((x - 1000.0).abs() < 1e-2);
    }

    #[test]
    fn time_and_x_are_inverse() {
        let mut state = UiState::default();
        state.horizontal_offset = 1e-4;
        for t in [-3e-4, 0.0, 1.5e-5, 2e-4] {
            let back = x_to_time(rect(), time_to_x(rect(), t, &state), &state);
            assert!((back - t).abs() < 1e-9, "t={t} back={back}");
        }
    }

    #[test]
    fn zero_volts_sits_at_centre_plus_offset() {
        let mut state = UiState::default();
        assert!((volts_to_y(rect(), 0.0, 0, &state) - 400.0).abs() < 1e-3);
        state.channels[0].offset_div = 1.0;
        assert!((volts_to_y(rect(), 0.0, 0, &state) - 300.0).abs() < 1e-3);
    }

    #[test]
    fn offset_of_one_division_moves_zero_up_by_one_division() {
        let mut state = UiState::default();
        let before = volts_to_y(rect(), 0.0, 0, &state);
        state.channels[0].offset_div += 1.0;
        let after = volts_to_y(rect(), 0.0, 0, &state);
        // 8 divisioni coprono 800 px, quindi una divisione vale 100 px, verso l'alto.
        assert!((before - after - 100.0).abs() < 1e-3);
    }

    #[test]
    fn y_grows_downwards_and_volts_scale_by_volts_per_div() {
        let state = UiState::default();
        let y0 = volts_to_y(rect(), 0.0, 0, &state);
        let y1 = volts_to_y(rect(), 1.0, 0, &state);
        assert!(y1 < y0);
        assert!((y0 - y1 - 100.0).abs() < 1e-3);
    }

    #[test]
    fn volts_and_y_are_inverse_on_both_channels() {
        let mut state = UiState::default();
        state.channels[0].volts_per_div = 0.5;
        for ch in 0..2 {
            for v in [-2.3f32, 0.0, 0.75, 3.1] {
                let back = y_to_volts(rect(), volts_to_y(rect(), v, ch, &state), ch, &state);
                assert!((back - v).abs() < 1e-3, "ch={ch} v={v} back={back}");
            }
        }
    }
}
