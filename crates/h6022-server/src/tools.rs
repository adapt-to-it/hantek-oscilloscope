//! Strumenti MCP. Ogni strumento è un metodo di `H6022Mcp` che lavora sullo stato condiviso.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, MutexGuard};
use std::time::{Duration, Instant};

use h6022_core::{EngineConfig, EngineHandle, Frame, RunState, Slope, TriggerKind, TriggerMode};
use h6022_dsp::measure::{Measurements, delay_seconds, measure as measure_wave, phase_degrees};
use h6022_dsp::resample::min_max_decimate;
use h6022_dsp::spectrum::spectrum as spectrum_of;
use h6022_dsp::{Waveform, Window};
use h6022_record::{Format, RecordLimit, RecordStatus, RecordSummary, Recorder};
use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ErrorData, Json, ServerHandler, tool, tool_handler, tool_router};

use crate::AppState;
use crate::api::{
    AnalysisView, AnalyzeParams, CaptureParams, ChannelMeasures, ConfigPatch, ConfigView, CouplingName, FormatName,
    FrameView, MeasureParams, MeasureView, ModeName, PeakView, RecordParams, RecordSummaryView, RecordingView,
    SignalKind, SlopeName, SnapshotParams, SpectrumParams, SpectrumView, StatusView, TraceView, WindowName,
};

#[derive(Clone)]
pub struct H6022Mcp {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

impl H6022Mcp {
    pub fn new(state: AppState) -> H6022Mcp {
        H6022Mcp { state, tool_router: Self::tool_router() + Self::decode_router() }
    }
}

#[tool_router(router = tool_router)]
impl H6022Mcp {
    #[tool(description = "Restituisce stato, contatori, configurazione e registrazione del dispositivo.")]
    pub async fn device_status(&self) -> Result<Json<StatusView>, ErrorData> {
        Ok(Json(self.status()?))
    }

    #[tool(description = "Restituisce la configurazione corrente del motore.")]
    pub async fn get_config(&self) -> Result<Json<ConfigView>, ErrorData> {
        Ok(Json(config_view(&self.state.engine.config())))
    }

    #[tool(description = "Modifica la configurazione; i campi assenti restano invariati.")]
    pub async fn set_config(&self, params: Parameters<ConfigPatch>) -> Result<Json<ConfigView>, ErrorData> {
        let patch = params.0;
        let mut cfg = self.state.engine.config();
        let touches_ch2 = patch.range2_v.is_some() || patch.coupling2.is_some() || patch.probe2.is_some();

        if let Some(hz) = patch.sample_rate_hz {
            if !(hz.is_finite() && hz > 0.0) {
                return Err(invalid("sample_rate_hz deve essere positiva"));
            }
            cfg.acquisition.sample_rate = SampleRate::nearest(hz);
        }
        if let Some(n) = patch.channels {
            cfg.acquisition.mode = channel_mode(n)?;
        }
        if let Some(v) = patch.range1_v {
            cfg.acquisition.range[0] = range_from_volts(v)?;
        }
        if let Some(v) = patch.range2_v {
            cfg.acquisition.range[1] = range_from_volts(v)?;
        }
        if let Some(c) = patch.coupling1 {
            cfg.acquisition.coupling[0] = coupling(c);
        }
        if let Some(c) = patch.coupling2 {
            cfg.acquisition.coupling[1] = coupling(c);
        }
        if let Some(p) = patch.probe1 {
            cfg.probe[0] = p;
        }
        if let Some(p) = patch.probe2 {
            cfg.probe[1] = p;
        }
        if let Some(m) = patch.mode {
            cfg.mode = trigger_mode(m);
        }
        if let Some(n) = patch.record_length {
            cfg.record_length = n;
        }
        if let Some(p) = patch.pretrigger {
            cfg.pretrigger = p;
        }
        if let Some(s) = patch.trigger_source {
            cfg.trigger.source = channel(s)?;
        }
        if let Some(s) = patch.trigger_slope {
            cfg.trigger.kind = TriggerKind::Edge { slope: slope(s) };
        }
        if let Some(l) = patch.trigger_level_v {
            cfg.trigger.level = l;
        }
        if let Some(h) = patch.trigger_hysteresis_v {
            cfg.trigger.hysteresis = h;
        }
        if let Some(h) = patch.trigger_holdoff_s {
            cfg.trigger.holdoff = h;
        }
        if cfg.acquisition.mode == ChannelMode::Single && (touches_ch2 || cfg.trigger.source == Channel::Ch2) {
            return Err(invalid("CH2 non attivo con un solo canale"));
        }

        self.state.engine.set_config(cfg).map_err(|e| invalid(e.to_string()))?;
        Ok(Json(config_view(&self.state.engine.config())))
    }

    #[tool(description = "Avvia l'acquisizione e restituisce lo stato del dispositivo.")]
    pub async fn run(&self) -> Result<Json<StatusView>, ErrorData> {
        self.state.engine.run();
        Ok(Json(self.status()?))
    }

    #[tool(description = "Ferma la pubblicazione dei frame e restituisce lo stato del dispositivo.")]
    pub async fn stop(&self) -> Result<Json<StatusView>, ErrorData> {
        self.state.engine.pause();
        Ok(Json(self.status()?))
    }

    #[tool(description = "Attende un frame con trigger in modalità singola e restituisce la traccia ridotta.")]
    pub async fn capture(&self, params: Parameters<CaptureParams>) -> Result<Json<FrameView>, ErrorData> {
        let timeout = timeout_secs(params.0.timeout_s, 5.0)?;
        let points = points_of(params.0.points)?;
        let engine = &self.state.engine;
        let previous = engine.config().mode;
        let rx = engine.subscribe();
        self.set_mode(TriggerMode::Single)?;
        engine.run();

        let outcome = match tokio::task::spawn_blocking(move || wait_triggered(&rx, timeout)).await {
            Ok(Some(frame)) => Ok(frame),
            Ok(None) => Err(internal(format!("nessun trigger entro {} s", timeout.as_secs_f64()))),
            Err(e) => Err(internal(e.to_string())),
        };
        let restored = self.restore(previous);
        let frame = outcome?;
        restored?;
        Ok(Json(frame_view(&frame, points)))
    }

    #[tool(description = "Restituisce l'ultimo frame pubblicato come traccia ridotta.")]
    pub async fn snapshot(&self, params: Parameters<SnapshotParams>) -> Result<Json<FrameView>, ErrorData> {
        let points = points_of(params.0.points)?;
        let frame = self.latest()?;
        Ok(Json(frame_view(&frame, points)))
    }

    #[tool(description = "Media le misure su più frame consecutivi per ogni canale.")]
    pub async fn measure(&self, params: Parameters<MeasureParams>) -> Result<Json<MeasureView>, ErrorData> {
        let n = frames_of(params.0.frames)?;
        let timeout = timeout_secs(params.0.timeout_s, 5.0)?;
        let rx = self.state.engine.subscribe();
        let frames = tokio::task::spawn_blocking(move || collect_frames(&rx, n, timeout))
            .await
            .map_err(|e| internal(e.to_string()))?;
        if frames.len() < n {
            return Err(internal(format!(
                "solo {} frame su {} entro {} s",
                frames.len(),
                n,
                timeout.as_secs_f64()
            )));
        }

        let last = Arc::clone(&frames[frames.len() - 1]);
        let mut per_channel: Vec<(u8, Vec<Measurements>)> = vec![(1, Vec::new())];
        if last.ch2.is_some() {
            per_channel.push((2, Vec::new()));
        }
        for frame in &frames {
            for (number, list) in per_channel.iter_mut() {
                if let Some(wave) = wave_by_number(frame, *number) {
                    list.push(measure_wave(wave));
                }
            }
        }
        let channels = per_channel.iter().map(|(number, list)| channel_measures(*number, list)).collect();
        let (phase_deg, delay_s) = match (&last.ch1, &last.ch2) {
            (a, Some(b)) => (phase_degrees(a, b), delay_seconds(a, b)),
            _ => (None, None),
        };
        Ok(Json(MeasureView { frames: n as u32, channels, phase_deg, delay_s }))
    }

    #[tool(description = "Calcola lo spettro di un canale sull'ultimo frame, con picchi e THD.")]
    pub async fn spectrum(&self, params: Parameters<SpectrumParams>) -> Result<Json<SpectrumView>, ErrorData> {
        let number = params.0.channel;
        let ch = channel(number)?;
        let window = params.0.window.unwrap_or(WindowName::FlatTop);
        let peaks_n = peaks_of(params.0.peaks)?;
        let frame = self.latest()?;
        let wave = wave_of(&frame, ch)?;
        let sp = spectrum_of(wave, window_of(window));
        let peaks: Vec<PeakView> = sp
            .peaks(peaks_n)
            .into_iter()
            .map(|pk| PeakView { frequency_hz: pk.frequency, vrms: pk.vrms, dbv: dbv(pk.vrms) })
            .collect();
        let thd_pct = peaks.first().and_then(|pk| sp.thd_pct(pk.frequency_hz));
        Ok(Json(SpectrumView {
            channel: number,
            window,
            bin_hz: sp.bin_hz,
            peaks,
            thd_pct,
        }))
    }

    #[tool(description = "Avvia la registrazione dei frame su disco in un formato csv, npy o wav.")]
    pub async fn start_recording(&self, params: Parameters<RecordParams>) -> Result<Json<RecordingView>, ErrorData> {
        let mut slot = self.recorder()?;
        if slot.is_some() {
            return Err(invalid("registrazione già attiva: chiama stop_recording prima"));
        }
        let recorder = start_recorder(&self.state.engine, params.0)?;
        let view = recording_view(recorder.status());
        *slot = Some(recorder);
        Ok(Json(view))
    }

    #[tool(description = "Ferma la registrazione in corso e restituisce il riepilogo.")]
    pub async fn stop_recording(&self) -> Result<Json<RecordSummaryView>, ErrorData> {
        let taken = self.recorder()?.take();
        let recorder = taken.ok_or_else(|| invalid("nessuna registrazione attiva"))?;
        let summary = recorder.stop().map_err(|e| internal(e.to_string()))?;
        Ok(Json(record_summary_view(summary)))
    }

    #[tool(description = "Classifica il segnale di un canale sull'ultimo frame e suggerisce base dei tempi e portata.")]
    pub async fn analyze_signal(&self, params: Parameters<AnalyzeParams>) -> Result<Json<AnalysisView>, ErrorData> {
        let number = params.0.channel;
        let ch = channel(number)?;
        let frame = self.latest()?;
        let wave = wave_of(&frame, ch)?;
        let m = measure_wave(wave);
        let thd = m.frequency.and_then(|f| spectrum_of(wave, Window::FlatTop).thd_pct(f));
        let kind = classify(&m, thd);
        let probe = frame.config.probe[usize::from(number - 1)];
        Ok(Json(AnalysisView {
            channel: number,
            kind,
            summary: describe(kind, &m),
            suggested_time_per_div_s: m.period.map(|p| 0.3 * p),
            suggested_range_v: suggested_range(&m, probe),
            measures: channel_measures(number, std::slice::from_ref(&m)),
            thd_pct: thd,
        }))
    }
}

impl H6022Mcp {
    fn status(&self) -> Result<StatusView, ErrorData> {
        let engine = &self.state.engine;
        let stats = engine.stats();
        let state = match engine.state() {
            RunState::Running => "running",
            RunState::Armed => "armed",
            RunState::Stopped => "stopped",
        };
        let guard = self.recorder()?;
        let recording = guard.as_ref().map(|r| recording_view(r.status()));
        Ok(StatusView {
            source: self.state.source.to_string(),
            state: state.to_string(),
            frames: stats.frames,
            triggers: stats.triggers,
            samples: stats.samples,
            sample_rate_measured_hz: stats.sample_rate_measured,
            config: config_view(&engine.config()),
            recording,
            websocket_url: format!("ws://{}/ws", self.state.addr),
        })
    }

    pub(crate) fn latest(&self) -> Result<Arc<Frame>, ErrorData> {
        self.state.engine.latest_frame().ok_or_else(|| internal("nessun frame disponibile"))
    }

    fn recorder(&self) -> Result<MutexGuard<'_, Option<Recorder>>, ErrorData> {
        self.state.recorder.lock().map_err(|_| internal("registrazione non disponibile"))
    }

    fn set_mode(&self, mode: TriggerMode) -> Result<(), ErrorData> {
        let engine = &self.state.engine;
        let mut cfg = engine.config();
        cfg.mode = mode;
        engine.set_config(cfg).map_err(|e| internal(e.to_string()))
    }

    /// Ripristina il modo di trigger; con un modo diverso da Single riprende l'acquisizione.
    fn restore(&self, previous: TriggerMode) -> Result<(), ErrorData> {
        self.set_mode(previous)?;
        if previous != TriggerMode::Single {
            self.state.engine.run();
        }
        Ok(())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for H6022Mcp {}

pub(crate) fn invalid(msg: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(msg.into(), None)
}

pub(crate) fn internal(msg: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(msg.into(), None)
}

pub(crate) fn channel(number: u8) -> Result<Channel, ErrorData> {
    match number {
        1 => Ok(Channel::Ch1),
        2 => Ok(Channel::Ch2),
        _ => Err(invalid("canale non valido: usa 1 o 2")),
    }
}

fn channel_mode(channels: u8) -> Result<ChannelMode, ErrorData> {
    match channels {
        1 => Ok(ChannelMode::Single),
        2 => Ok(ChannelMode::Dual),
        _ => Err(invalid("channels deve essere 1 o 2")),
    }
}

fn range_from_volts(volts: f32) -> Result<VoltageRange, ErrorData> {
    VoltageRange::all()
        .into_iter()
        .find(|r| (r.half_span_volts() - volts).abs() < 1e-6)
        .ok_or_else(|| invalid("portata non valida: usa 5, 2.5, 1 o 0.5 V"))
}

pub(crate) fn wave_of(frame: &Frame, ch: Channel) -> Result<&Waveform, ErrorData> {
    match ch {
        Channel::Ch1 => Ok(&frame.ch1),
        Channel::Ch2 => frame.ch2.as_ref().ok_or_else(|| invalid("CH2 non attivo: la modalità ha un solo canale")),
    }
}

fn wave_by_number(frame: &Frame, number: u8) -> Option<&Waveform> {
    match number {
        1 => Some(&frame.ch1),
        2 => frame.ch2.as_ref(),
        _ => None,
    }
}

fn points_of(value: Option<usize>) -> Result<usize, ErrorData> {
    let n = value.unwrap_or(500);
    if n == 0 || n > 5000 {
        return Err(invalid("points deve essere tra 1 e 5000"));
    }
    Ok(n)
}

fn peaks_of(value: Option<usize>) -> Result<usize, ErrorData> {
    let n = value.unwrap_or(10);
    if n == 0 || n > 50 {
        return Err(invalid("peaks deve essere tra 1 e 50"));
    }
    Ok(n)
}

fn frames_of(value: Option<u32>) -> Result<usize, ErrorData> {
    let n = value.unwrap_or(1);
    if !(1..=50).contains(&n) {
        return Err(invalid("frames deve essere tra 1 e 50"));
    }
    Ok(n as usize)
}

fn timeout_secs(value: Option<f64>, default: f64) -> Result<Duration, ErrorData> {
    let s = value.unwrap_or(default);
    if !(s > 0.0 && s <= 60.0) {
        return Err(invalid("timeout_s deve essere tra 0 e 60"));
    }
    Ok(Duration::from_secs_f64(s))
}

/// Attende un frame con trigger; `None` se scade il tempo o il motore si chiude.
fn wait_triggered(rx: &Receiver<Arc<Frame>>, timeout: Duration) -> Option<Arc<Frame>> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match rx.recv_timeout(left) {
            Ok(frame) if frame.triggered => return Some(frame),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

/// Raccoglie fino a `n` frame; ne restituisce meno se scade il tempo.
fn collect_frames(rx: &Receiver<Arc<Frame>>, n: usize, timeout: Duration) -> Vec<Arc<Frame>> {
    let deadline = Instant::now() + timeout;
    let mut frames = Vec::with_capacity(n);
    while frames.len() < n {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match rx.recv_timeout(left) {
            Ok(frame) => frames.push(frame),
            Err(_) => break,
        }
    }
    frames
}

fn trace(channel: u8, wave: &Waveform, points: usize) -> TraceView {
    let pairs = min_max_decimate(&wave.samples, points);
    TraceView {
        channel,
        min_v: pairs.iter().map(|p| p.0).collect(),
        max_v: pairs.iter().map(|p| p.1).collect(),
    }
}

fn frame_view(frame: &Frame, points: usize) -> FrameView {
    let rate = frame.ch1.sample_rate;
    let n = frame.ch1.samples.len();
    let points = points.min(frame.config.record_length);
    let mut channels = vec![trace(1, &frame.ch1, points)];
    if let Some(ch2) = &frame.ch2 {
        channels.push(trace(2, ch2, points));
    }
    FrameView {
        sequence: frame.sequence,
        triggered: frame.triggered,
        sample_rate_hz: rate,
        record_length: frame.config.record_length,
        t_start_s: frame.ch1.t0,
        t_end_s: frame.ch1.t0 + n.saturating_sub(1) as f64 / rate,
        points: channels.first().map_or(0, |t| t.min_v.len()),
        channels,
    }
}

fn mean_of(ms: &[Measurements], f: impl Fn(&Measurements) -> Option<f64>) -> Option<f64> {
    let (sum, count) = ms.iter().filter_map(|m| f(m)).fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    (count > 0).then(|| sum / count as f64)
}

fn mean_req(ms: &[Measurements], f: impl Fn(&Measurements) -> f64) -> f64 {
    mean_of(ms, |m| Some(f(m))).unwrap_or(0.0)
}

fn channel_measures(channel: u8, ms: &[Measurements]) -> ChannelMeasures {
    ChannelMeasures {
        channel,
        vpp: mean_req(ms, |m| f64::from(m.vpp)) as f32,
        vmax: mean_req(ms, |m| f64::from(m.vmax)) as f32,
        vmin: mean_req(ms, |m| f64::from(m.vmin)) as f32,
        vmean: mean_req(ms, |m| f64::from(m.vmean)) as f32,
        vrms: mean_req(ms, |m| f64::from(m.vrms)) as f32,
        vac_rms: mean_req(ms, |m| f64::from(m.vac_rms)) as f32,
        vtop: mean_req(ms, |m| f64::from(m.vtop)) as f32,
        vbase: mean_req(ms, |m| f64::from(m.vbase)) as f32,
        vamp: mean_req(ms, |m| f64::from(m.vamp)) as f32,
        frequency_hz: mean_of(ms, |m| m.frequency),
        period_s: mean_of(ms, |m| m.period),
        duty_pct: mean_of(ms, |m| m.duty_positive.map(|d| d * 100.0)),
        pulse_width_pos_s: mean_of(ms, |m| m.pulse_width_positive),
        pulse_width_neg_s: mean_of(ms, |m| m.pulse_width_negative),
        rise_s: mean_of(ms, |m| m.rise_time),
        fall_s: mean_of(ms, |m| m.fall_time),
        overshoot_pct: mean_of(ms, |m| m.overshoot_pct.map(f64::from)).map(|v| v as f32),
        preshoot_pct: mean_of(ms, |m| m.preshoot_pct.map(f64::from)).map(|v| v as f32),
    }
}

fn dbv(vrms: f32) -> f32 {
    if vrms > 0.0 { (20.0 * vrms.log10()).max(-200.0) } else { -200.0 }
}

fn window_of(name: WindowName) -> Window {
    match name {
        WindowName::Rectangular => Window::Rectangular,
        WindowName::Hann => Window::Hann,
        WindowName::Hamming => Window::Hamming,
        WindowName::BlackmanHarris => Window::BlackmanHarris,
        WindowName::FlatTop => Window::FlatTop,
    }
}

/// Classifica il segnale con le regole in ordine: continua, rumore, sinusoide, quadra, PWM, altro.
fn classify(m: &Measurements, thd: Option<f64>) -> SignalKind {
    if m.vac_rms < 0.02 {
        return SignalKind::Dc;
    }
    if m.frequency.is_none() {
        return SignalKind::Noise;
    }
    if thd.is_some_and(|t| t < 5.0) {
        return SignalKind::Sine;
    }
    let fast = match (m.rise_time, m.fall_time, m.period) {
        (Some(r), Some(f), Some(p)) => r + f < 0.1 * p,
        _ => false,
    };
    if !fast {
        return SignalKind::Other;
    }
    let duty = m.duty_positive.map(|d| d * 100.0);
    if duty.is_some_and(|d| (45.0..=55.0).contains(&d)) {
        SignalKind::Square
    } else {
        SignalKind::Pwm
    }
}

/// Numeri con virgola decimale, come si scrivono in italiano.
fn decimal(value: f64, digits: usize) -> String {
    format!("{:.*}", digits, value).replace('.', ",")
}

fn frequency_label(hz: f64) -> String {
    if hz >= 1000.0 {
        format!("{} kHz", decimal(hz / 1000.0, 2))
    } else {
        format!("{} Hz", decimal(hz, 1))
    }
}

fn describe(kind: SignalKind, m: &Measurements) -> String {
    let name = match kind {
        SignalKind::Dc => "Livello continuo",
        SignalKind::Noise => "Rumore",
        SignalKind::Square => "Onda quadra",
        SignalKind::Pwm => "PWM",
        SignalKind::Sine => "Sinusoide",
        SignalKind::Other => "Segnale",
    };
    let mut parts = Vec::new();
    if let Some(f) = m.frequency {
        parts.push(frequency_label(f));
    }
    parts.push(format!("{}–{} V", decimal(f64::from(m.vbase), 2), decimal(f64::from(m.vtop), 2)));
    if matches!(kind, SignalKind::Square | SignalKind::Pwm) {
        if let Some(d) = m.duty_positive {
            parts.push(format!("duty {} %", decimal(d * 100.0, 0)));
        }
    }
    format!("{name} {}", parts.join(", "))
}

/// Portata più piccola che contiene il segnale con margine del 10 %, altrimenti 5 V.
fn suggested_range(m: &Measurements, probe: f32) -> f32 {
    let needed = m.vmax.abs().max(m.vmin.abs()) / probe;
    [0.5f32, 1.0, 2.5, 5.0].into_iter().find(|&r| needed <= 0.9 * r).unwrap_or(5.0)
}

fn format_of(name: FormatName) -> Format {
    match name {
        FormatName::Csv => Format::Csv,
        FormatName::Npy => Format::Npy,
        FormatName::Wav => Format::Wav,
    }
}

fn format_name(format: Format) -> FormatName {
    match format {
        Format::Csv => FormatName::Csv,
        Format::Npy => FormatName::Npy,
        Format::Wav => FormatName::Wav,
    }
}

/// Valida i parametri e avvia una registrazione; la usano lo strumento MCP e l'app desktop.
pub fn start_recorder(engine: &EngineHandle, p: RecordParams) -> Result<Recorder, ErrorData> {
    if p.max_seconds.is_some_and(|s| !(s > 0.0 && s.is_finite())) {
        return Err(invalid("max_seconds deve essere positivo"));
    }
    let (path, format) = recording_target(&p.path, p.format)?;
    let limit = RecordLimit { max_frames: p.max_frames, max_seconds: p.max_seconds };
    Recorder::start(engine, &path, format, limit).map_err(record_error)
}

/// Percorso assoluto del file e formato. Un percorso relativo va in `$HOME/h6022-recordings`.
pub(crate) fn recording_target(raw: &str, format: Option<FormatName>) -> Result<(PathBuf, Format), ErrorData> {
    if raw.trim().is_empty() {
        return Err(invalid("percorso vuoto"));
    }
    let mut path = PathBuf::from(raw);
    let explicit = format.map(format_of);
    let format = match (explicit, Format::from_path(&path)) {
        (Some(f), Some(e)) if f != e => {
            return Err(invalid("l'estensione del percorso non corrisponde al formato"));
        }
        (Some(_), None) if path.extension().is_some() => {
            return Err(invalid("l'estensione del percorso non corrisponde al formato"));
        }
        (Some(f), _) => f,
        (None, Some(e)) => e,
        (None, None) => {
            return Err(invalid("formato non riconosciuto: indica format o un'estensione csv, npy o wav"));
        }
    };
    if path.extension().is_none() {
        path.set_extension(format.extension());
    }
    if !path.is_absolute() {
        let home = std::env::var_os("HOME").ok_or_else(|| internal("HOME non impostata"))?;
        let dir = PathBuf::from(home).join("h6022-recordings");
        std::fs::create_dir_all(&dir).map_err(|e| internal(format!("cartella {}: {e}", dir.display())))?;
        path = dir.join(path);
    }
    Ok((path, format))
}

fn record_error(err: std::io::Error) -> ErrorData {
    match err.kind() {
        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::InvalidInput => invalid(err.to_string()),
        _ => internal(err.to_string()),
    }
}

pub fn recording_view(status: RecordStatus) -> RecordingView {
    RecordingView {
        path: status.path.to_string_lossy().into_owned(),
        format: format_name(status.format),
        frames: status.frames,
        running: status.running,
        error: status.error,
    }
}

pub fn record_summary_view(summary: RecordSummary) -> RecordSummaryView {
    RecordSummaryView {
        path: summary.path.to_string_lossy().into_owned(),
        format: format_name(summary.format),
        frames: summary.frames,
        channels: summary.channels,
        sample_rate_hz: summary.sample_rate_hz,
        samples_per_channel: summary.samples_per_channel,
    }
}

fn config_view(c: &EngineConfig) -> ConfigView {
    let a: &AcquisitionConfig = &c.acquisition;
    let trigger_slope = match c.trigger.kind {
        TriggerKind::Edge { slope } | TriggerKind::Timeout { slope, .. } => Some(slope_name(slope)),
        _ => None,
    };
    let trigger_kind = match c.trigger.kind {
        TriggerKind::Edge { .. } => "edge",
        TriggerKind::PulseWidth { .. } => "pulse_width",
        TriggerKind::Runt { .. } => "runt",
        TriggerKind::Window { .. } => "window",
        TriggerKind::Timeout { .. } => "timeout",
        TriggerKind::Pattern { .. } => "pattern",
    };
    ConfigView {
        sample_rate_hz: f64::from(a.sample_rate.hz()),
        channels: match a.mode {
            ChannelMode::Single => 1,
            ChannelMode::Dual => 2,
        },
        range_v: [a.range[0].half_span_volts(), a.range[1].half_span_volts()],
        coupling: [coupling_name(a.coupling[0]), coupling_name(a.coupling[1])],
        probe: c.probe,
        mode: mode_name(c.mode),
        record_length: c.record_length,
        pretrigger: c.pretrigger,
        trigger_source: match c.trigger.source {
            Channel::Ch1 => 1,
            Channel::Ch2 => 2,
        },
        trigger_kind: trigger_kind.to_string(),
        trigger_slope,
        trigger_level_v: c.trigger.level,
        trigger_hysteresis_v: c.trigger.hysteresis,
        trigger_holdoff_s: c.trigger.holdoff,
    }
}

fn coupling(name: CouplingName) -> Coupling {
    match name {
        CouplingName::Dc => Coupling::Dc,
        CouplingName::Ac => Coupling::Ac,
    }
}

fn coupling_name(c: Coupling) -> CouplingName {
    match c {
        Coupling::Dc => CouplingName::Dc,
        Coupling::Ac => CouplingName::Ac,
    }
}

fn trigger_mode(name: ModeName) -> TriggerMode {
    match name {
        ModeName::Auto => TriggerMode::Auto,
        ModeName::Normal => TriggerMode::Normal,
        ModeName::Single => TriggerMode::Single,
        ModeName::Roll => TriggerMode::Roll,
    }
}

fn mode_name(mode: TriggerMode) -> ModeName {
    match mode {
        TriggerMode::Auto => ModeName::Auto,
        TriggerMode::Normal => ModeName::Normal,
        TriggerMode::Single => ModeName::Single,
        TriggerMode::Roll => ModeName::Roll,
    }
}

fn slope(name: SlopeName) -> Slope {
    match name {
        SlopeName::Rising => Slope::Rising,
        SlopeName::Falling => Slope::Falling,
        SlopeName::Either => Slope::Either,
    }
}

fn slope_name(slope: Slope) -> SlopeName {
    match slope {
        Slope::Rising => SlopeName::Rising,
        Slope::Falling => SlopeName::Falling,
        Slope::Either => SlopeName::Either,
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Mutex;

    use h6022_core::{Engine, TriggerConfig};
    use h6022_core::sim::SimulatedSource;

    use super::*;

    fn test_config() -> EngineConfig {
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
                hysteresis: 0.1,
                holdoff: 0.0,
            },
            mode: TriggerMode::Auto,
            record_length: 2000,
            pretrigger: 0.5,
            probe: [1.0, 1.0],
        }
    }

    fn state() -> AppState {
        let source = Box::new(SimulatedSource::bench_default());
        let engine = Engine::start(source, test_config()).unwrap();
        AppState {
            engine,
            source: "simulator",
            addr: SocketAddr::from(([127, 0, 0, 1], 6022)),
            recorder: Arc::new(Mutex::new(None)),
        }
    }

    /// Attende il primo frame, così `latest_frame` è popolato.
    fn first_frame(state: &AppState) {
        let rx = state.engine.subscribe();
        rx.recv_timeout(Duration::from_secs(5)).expect("nessun frame dal motore simulato");
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn set_config_e_get_config_coincidono() {
        let mcp = H6022Mcp::new(state());
        let patch = ConfigPatch { sample_rate_hz: Some(2e6), range1_v: Some(1.0), ..Default::default() };
        let set = block_on(mcp.set_config(Parameters(patch))).unwrap().0;
        assert_eq!(set.range_v[0], 1.0);
        assert_eq!(set.sample_rate_hz, 2e6);
        let got = block_on(mcp.get_config()).unwrap().0;
        assert_eq!(got, set);
    }

    #[test]
    fn snapshot_con_100_punti() {
        let st = state();
        first_frame(&st);
        let mcp = H6022Mcp::new(st);
        let view = block_on(mcp.snapshot(Parameters(SnapshotParams { points: Some(100) }))).unwrap().0;
        assert_eq!(view.points, 100);
        assert_eq!(view.channels[0].min_v.len(), 100);
        assert!(view.t_end_s > view.t_start_s);
    }

    #[test]
    fn measure_su_due_frame() {
        let st = state();
        first_frame(&st);
        let mcp = H6022Mcp::new(st);
        let view = block_on(mcp.measure(Parameters(MeasureParams { frames: Some(2), timeout_s: None }))).unwrap().0;
        assert_eq!(view.frames, 2);
        assert_eq!(view.channels[0].channel, 1);
        assert!(view.channels[0].vpp > 2.0);
    }

    #[test]
    fn analyze_riconosce_l_onda_quadra_simulata() {
        let st = state();
        first_frame(&st);
        let mcp = H6022Mcp::new(st);
        let view = block_on(mcp.analyze_signal(Parameters(AnalyzeParams { channel: 1 }))).unwrap().0;
        assert_eq!(view.kind, SignalKind::Square);
        assert!(view.summary.contains("kHz"));
    }

    #[test]
    fn stop_recording_senza_registrazione_e_errore() {
        let mcp = H6022Mcp::new(state());
        assert!(block_on(mcp.stop_recording()).is_err());
    }

    #[test]
    fn start_e_stop_recording_in_cartella_temporanea() {
        let dir = std::env::temp_dir().join(format!("h6022-mcp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let st = state();
        first_frame(&st);
        let mcp = H6022Mcp::new(st);
        let path = dir.join("rec.csv").to_string_lossy().into_owned();
        let started = block_on(mcp.start_recording(Parameters(RecordParams {
            path,
            format: None,
            max_frames: Some(2),
            max_seconds: None,
        })))
        .unwrap()
        .0;
        assert_eq!(started.format, FormatName::Csv);
        let summary = block_on(mcp.stop_recording()).unwrap().0;
        assert_eq!(summary.format, FormatName::Csv);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
