//! WebSocket binario `/ws`: un messaggio binario per frame.
//!
//! Intestazione little-endian di `HEADER_LEN` byte:
//! - 0..8   magia `b"H6022WS1"`
//! - 8..16  sequence u64
//! - 16..24 sample_rate_hz f64
//! - 24..32 t0_s f64 (tempo del primo campione rispetto al trigger)
//! - 32..36 campioni per canale u32
//! - 36     canali u8 (1 o 2)
//! - 37     triggered u8 (0 o 1)
//! - 38..40 riservati, zero
//!
//! Seguono i campioni f32 di CH1, poi quelli di CH2 se presente.
//! Query `?max_fps=N` (1..=120, predefinito 30): i frame in eccesso vengono saltati.
//! I messaggi del client vengono ignorati; un Close chiude la connessione.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use axum::Extension;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use h6022_core::Frame;
use serde::Deserialize;
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio_util::sync::CancellationToken;

use crate::AppState;

pub const MAGIC: &[u8; 8] = b"H6022WS1";
pub const HEADER_LEN: usize = 40;

const DEFAULT_MAX_FPS: u32 = 30;
const MAX_FPS_LIMIT: u32 = 120;
/// Coda tra il thread di lettura e il task WebSocket: se è piena il frame si scarta.
const QUEUE: usize = 2;
/// Intervallo con cui il thread di lettura controlla se la connessione è ancora aperta.
const POLL: Duration = Duration::from_millis(200);

#[derive(Deserialize)]
pub struct WsQuery {
    pub max_fps: Option<u32>,
}

pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let channels: u8 = if frame.ch2.is_some() { 2 } else { 1 };
    let samples = frame.ch1.samples.len();
    let mut out = Vec::with_capacity(HEADER_LEN + 4 * samples * usize::from(channels));
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&frame.sequence.to_le_bytes());
    out.extend_from_slice(&frame.ch1.sample_rate.to_le_bytes());
    out.extend_from_slice(&frame.ch1.t0.to_le_bytes());
    out.extend_from_slice(&(samples as u32).to_le_bytes());
    out.push(channels);
    out.push(u8::from(frame.triggered));
    out.extend_from_slice(&[0, 0]);
    for s in &frame.ch1.samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    if let Some(ch2) = &frame.ch2 {
        for s in &ch2.samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    out
}

/// Handler di `/ws`: valida `max_fps` prima dell'upgrade, così un valore errato risponde 400.
pub async fn upgrade(
    State(state): State<AppState>,
    Extension(stop): Extension<CancellationToken>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let max_fps = query.max_fps.unwrap_or(DEFAULT_MAX_FPS);
    if !(1..=MAX_FPS_LIMIT).contains(&max_fps) {
        return (StatusCode::BAD_REQUEST, "max_fps deve essere tra 1 e 120").into_response();
    }
    ws.on_upgrade(move |socket| session(socket, state, max_fps, stop))
}

async fn session(mut socket: WebSocket, state: AppState, max_fps: u32, stop: CancellationToken) {
    let (tx, mut rx) = mpsc::channel::<Arc<Frame>>(QUEUE);
    let frames = state.engine.subscribe();
    thread::spawn(move || pump(frames, tx));

    let min_gap = Duration::from_secs_f64(1.0 / f64::from(max_fps));
    let mut last_sent: Option<Instant> = None;
    loop {
        tokio::select! {
            _ = stop.cancelled() => break,
            frame = rx.recv() => {
                let Some(frame) = frame else { break };
                let now = Instant::now();
                if last_sent.is_some_and(|t| now.duration_since(t) < min_gap) {
                    continue;
                }
                last_sent = Some(now);
                if socket.send(Message::Binary(encode_frame(&frame).into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => match msg {
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

/// Inoltra i frame del motore alla coda async senza bloccarsi. Esce quando il canale si chiude.
fn pump(frames: Receiver<Arc<Frame>>, tx: mpsc::Sender<Arc<Frame>>) {
    loop {
        match frames.recv_timeout(POLL) {
            Ok(frame) => match tx.try_send(frame) {
                Ok(()) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Closed(_)) => break,
            },
            Err(RecvTimeoutError::Timeout) => {
                if tx.is_closed() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use h6022_core::{EngineConfig, Frame, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_dsp::Waveform;
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::*;

    fn config() -> EngineConfig {
        EngineConfig {
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
                hysteresis: 0.0,
                holdoff: 0.0,
            },
            mode: TriggerMode::Normal,
            record_length: 3,
            pretrigger: 0.0,
            probe: [1.0, 1.0],
        }
    }

    fn wave(samples: Vec<f32>) -> Waveform {
        Waveform { samples, sample_rate: 1e6, t0: -0.000_25 }
    }

    fn frame(ch2: Option<Vec<f32>>) -> Frame {
        Frame {
            sequence: 7,
            ch1: wave(vec![1.0, 2.0, 3.0]),
            ch2: ch2.map(wave),
            triggered: true,
            trigger_sample: Some(10),
            config: config(),
        }
    }

    fn le_u64(b: &[u8]) -> u64 {
        u64::from_le_bytes(b.try_into().unwrap())
    }

    fn le_f64(b: &[u8]) -> f64 {
        f64::from_le_bytes(b.try_into().unwrap())
    }

    fn le_f32(b: &[u8]) -> f32 {
        f32::from_le_bytes(b.try_into().unwrap())
    }

    #[test]
    fn frame_a_due_canali() {
        let bytes = encode_frame(&frame(Some(vec![4.0, 5.0, 6.0])));
        assert_eq!(bytes.len(), HEADER_LEN + 4 * 3 * 2);
        assert_eq!(&bytes[0..8], MAGIC);
        assert_eq!(le_u64(&bytes[8..16]), 7);
        assert_eq!(le_f64(&bytes[16..24]), 1e6);
        assert_eq!(le_f64(&bytes[24..32]), -0.000_25);
        assert_eq!(u32::from_le_bytes(bytes[32..36].try_into().unwrap()), 3);
        assert_eq!(bytes[36], 2);
        assert_eq!(bytes[37], 1);
        assert_eq!(&bytes[38..40], &[0, 0]);
        assert_eq!(le_f32(&bytes[HEADER_LEN..HEADER_LEN + 4]), 1.0);
        assert_eq!(le_f32(&bytes[HEADER_LEN + 12..HEADER_LEN + 16]), 4.0);
    }

    #[test]
    fn frame_a_un_canale() {
        let mut f = frame(None);
        f.triggered = false;
        let bytes = encode_frame(&f);
        assert_eq!(bytes.len(), HEADER_LEN + 4 * 3);
        assert_eq!(&bytes[0..8], MAGIC);
        assert_eq!(bytes[36], 1);
        assert_eq!(bytes[37], 0);
        assert_eq!(le_f32(&bytes[HEADER_LEN + 8..HEADER_LEN + 12]), 3.0);
    }
}
