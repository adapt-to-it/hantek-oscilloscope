//! Strumenti MCP dei decoder. Ogni strumento lavora sull'ultimo frame, come `snapshot` e `analyze_signal`.

use std::fs::OpenOptions;
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};

use h6022_core::Frame;
use h6022_decode::can::CanConfig;
use h6022_decode::classify::{SignalClass, classify, segment};
use h6022_decode::export::{write_sigrok, write_vcd};
use h6022_decode::protocol::{Protocol, decode as run_decode, summary as protocol_summary};
use h6022_decode::search::find;
use h6022_decode::uart::{Parity, UartConfig};
use h6022_decode::{Annotation, LogicTrace, digitize};
use h6022_dsp::Waveform;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ErrorData, Json, tool, tool_router};

use crate::api_decode::{
    AnnotationView, ChannelParams, ClassificationView, DecodeParams, DecodeView, ExportFormatName, ExportParams,
    ExportView, MatchView, MatchesView, ParityName, PatternParams, SegmentView, SegmentsParams, SegmentsView,
};
use crate::tools::{H6022Mcp, channel, internal, invalid, wave_of};

#[tool_router(router = decode_router, vis = "pub(crate)")]
impl H6022Mcp {
    #[tool(description = "Decodifica un protocollo sull'ultimo frame e restituisce le annotazioni con il riepilogo.")]
    pub async fn decode(&self, params: Parameters<DecodeParams>) -> Result<Json<DecodeView>, ErrorData> {
        let frame = self.latest()?;
        decode_view(&frame, params.0).map(Json)
    }

    #[tool(description = "Classifica il segnale di un canale sull'ultimo frame e suggerisce i decoder adatti.")]
    pub async fn classify_signal(
        &self,
        params: Parameters<ChannelParams>,
    ) -> Result<Json<ClassificationView>, ErrorData> {
        let ch = channel(params.0.channel)?;
        let frame = self.latest()?;
        Ok(Json(classification_view(wave_of(&frame, ch)?)))
    }

    #[tool(description = "Trova i pacchetti di un canale sull'ultimo frame, separati da pause di linea costante.")]
    pub async fn find_packets(&self, params: Parameters<SegmentsParams>) -> Result<Json<SegmentsView>, ErrorData> {
        let p = params.0;
        let ch = channel(p.channel)?;
        if p.min_gap_s.is_some_and(|g| !(g.is_finite() && g > 0.0)) {
            return Err(invalid("min_gap_s deve essere positivo"));
        }
        let frame = self.latest()?;
        let wave = wave_of(&frame, ch)?;
        let trace = digitize(wave, None);
        let segments = segment(&trace, p.min_gap_s)
            .into_iter()
            .map(|s| SegmentView { start_s: s.start_s, end_s: s.end_s, transitions: s.transitions })
            .collect();
        Ok(Json(SegmentsView { segments }))
    }

    #[tool(description = "Cerca un tratto del canale sull'ultimo frame e restituisce le posizioni dove ricorre.")]
    pub async fn find_pattern(&self, params: Parameters<PatternParams>) -> Result<Json<MatchesView>, ErrorData> {
        let p = params.0;
        let ch = channel(p.channel)?;
        let min_score = p.min_score.unwrap_or(0.8);
        if !(-1.0..=1.0).contains(&min_score) {
            return Err(invalid("min_score deve essere tra -1 e 1"));
        }
        let max_matches = p.max_matches.unwrap_or(20);
        if !(1..=200).contains(&max_matches) {
            return Err(invalid("max_matches deve essere tra 1 e 200"));
        }
        let frame = self.latest()?;
        let wave = wave_of(&frame, ch)?;
        let template = template_of(wave, p.start_s, p.end_s)?;
        let matches = find(wave, &template, min_score, max_matches)
            .into_iter()
            .map(|m| MatchView { time_s: m.time_s, score: m.score })
            .collect();
        Ok(Json(MatchesView { matches }))
    }

    #[tool(description = "Esporta i canali attivi digitalizzati in un file vcd o sigrok sr; nel vcd entrano anche le annotazioni.")]
    pub async fn export_logic(&self, params: Parameters<ExportParams>) -> Result<Json<ExportView>, ErrorData> {
        let frame = self.latest()?;
        Ok(Json(export_view(&frame, params.0)?))
    }
}

/// Esporta in vcd o sr i canali del frame digitalizzati; la usano lo strumento MCP e l'app desktop.
/// Classificazione di una forma d'onda, condivisa con l'app desktop.
pub fn classification_view(wave: &Waveform) -> ClassificationView {
    let c = classify(wave);
    ClassificationView {
        class: class_name(&c.class).to_string(),
        confidence: c.confidence,
        summary: c.summary,
        baud_estimate: c.baud_estimate,
        suggested_decoders: c.suggested_decoders.into_iter().map(str::to_string).collect(),
    }
}

pub fn export_view(frame: &Frame, p: ExportParams) -> Result<ExportView, ErrorData> {
    {
        let path = export_target(&p.path, p.format)?;
        let threshold = threshold_of(p.decode.as_ref().and_then(|d| d.threshold_v))?;
        let annotations = match (p.format, &p.decode) {
            (ExportFormatName::Vcd, Some(d)) => decode_frame(frame, d)?,
            _ => Vec::new(),
        };
        let ch1 = digitize(&frame.ch1, threshold);
        let ch2 = frame.ch2.as_ref().map(|w| digitize(w, threshold));
        let mut traces: Vec<(&str, &LogicTrace)> = vec![("CH1", &ch1)];
        if let Some(t) = &ch2 {
            traces.push(("CH2", t));
        }
        let bytes = write_export(&path, p.format, &traces, &annotations)?;
        Ok(ExportView { path: path.to_string_lossy().into_owned(), bytes })
    }
}

fn annotations_of(value: Option<usize>) -> Result<usize, ErrorData> {
    let n = value.unwrap_or(200);
    if !(1..=5000).contains(&n) {
        return Err(invalid("max_annotations deve essere tra 1 e 5000"));
    }
    Ok(n)
}

fn positive(value: f64, name: &str) -> Result<f64, ErrorData> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(invalid(format!("{name} deve essere positivo")))
    }
}

fn threshold_of(value: Option<f32>) -> Result<Option<f32>, ErrorData> {
    match value {
        Some(v) if !v.is_finite() => Err(invalid("threshold_v deve essere un numero finito")),
        other => Ok(other),
    }
}

fn parity_of(name: ParityName) -> Parity {
    match name {
        ParityName::None => Parity::None,
        ParityName::Even => Parity::Even,
        ParityName::Odd => Parity::Odd,
    }
}

/// Converte la richiesta nel decoder di `h6022-decode`, con i suoi parametri.
fn protocol_of(p: &DecodeParams) -> Result<Protocol, ErrorData> {
    Ok(match p.protocol.as_str() {
        "uart" => {
            let data_bits = p.data_bits.unwrap_or(8);
            if !(5..=9).contains(&data_bits) {
                return Err(invalid("data_bits deve essere tra 5 e 9"));
            }
            let stop_bits = p.stop_bits.unwrap_or(1.0);
            if ![1.0, 1.5, 2.0].contains(&stop_bits) {
                return Err(invalid("stop_bits deve essere 1, 1.5 o 2"));
            }
            Protocol::Uart(UartConfig {
                baud: p.baud.map(|b| positive(b, "baud")).transpose()?,
                data_bits,
                parity: parity_of(p.parity.unwrap_or(ParityName::None)),
                stop_bits,
                inverted: p.inverted.unwrap_or(false),
                ..UartConfig::default()
            })
        }
        "i2c" => Protocol::I2c,
        "can" => {
            let bitrate = p.bitrate.ok_or_else(|| invalid("bitrate obbligatorio per can"))?;
            Protocol::Can(CanConfig { bitrate: positive(bitrate, "bitrate")?, inverted: p.inverted.unwrap_or(false) })
        }
        "onewire" => Protocol::OneWire,
        "manchester" => Protocol::Manchester {
            bit_period_s: p.bit_period_s.map(|t| positive(t, "bit_period_s")).transpose()?,
            ieee: p.ieee.unwrap_or(true),
        },
        "pwm" => Protocol::Pwm,
        "servo" => Protocol::Servo,
        "ir_nec" => Protocol::IrNec,
        "ir_rc5" => Protocol::IrRc5,
        other => return Err(invalid(format!("protocollo sconosciuto: {other}"))),
    })
}

/// Decodifica il canale richiesto dell'ultimo frame; per i2c l'altro canale è SDA.
/// Decodifica un frame e prepara la risposta: la usano lo strumento MCP `decode` e l'app desktop.
pub fn decode_view(frame: &Frame, p: DecodeParams) -> Result<DecodeView, ErrorData> {
    let limit = annotations_of(p.max_annotations)?;
    let all = decode_frame(frame, &p)?;
    let summary = protocol_summary(&all);
    let truncated = all.len() > limit;
    let annotations = all.iter().take(limit).map(annotation_view).collect();
    Ok(DecodeView { protocol: p.protocol, channel: p.channel, annotations, truncated, summary })
}

fn decode_frame(frame: &Frame, p: &DecodeParams) -> Result<Vec<Annotation>, ErrorData> {
    let protocol = protocol_of(p)?;
    let ch = channel(p.channel)?;
    let threshold = threshold_of(p.threshold_v)?;
    let primary = wave_of(frame, ch)?;
    let secondary = if p.channel == 1 { frame.ch2.as_ref() } else { Some(&frame.ch1) };
    run_decode(&protocol, primary, secondary, threshold).map_err(invalid)
}

fn annotation_view(a: &Annotation) -> AnnotationView {
    AnnotationView {
        start_s: a.start_s,
        end_s: a.end_s,
        kind: a.kind.to_string(),
        text: a.text.clone(),
        value: a.value,
        error: a.error,
    }
}

/// Nome minuscolo della classe, come nella risposta MCP.
fn class_name(class: &SignalClass) -> &'static str {
    match class {
        SignalClass::Dc => "dc",
        SignalClass::Noise => "noise",
        SignalClass::Analog => "analog",
        SignalClass::Clock => "clock",
        SignalClass::Pwm => "pwm",
        SignalClass::Serial => "serial",
        SignalClass::Burst => "burst",
        SignalClass::Other => "other",
    }
}

/// Tratto `[start_s, end_s]` del canale come forma d'onda. Vuoto o fuori dal frame è errore.
fn template_of(wave: &Waveform, start_s: f64, end_s: f64) -> Result<Waveform, ErrorData> {
    let n = wave.samples.len();
    let last_s = wave.t0 + n.saturating_sub(1) as f64 / wave.sample_rate;
    let inside = start_s.is_finite() && end_s.is_finite() && start_s >= wave.t0 && end_s <= last_s;
    if !inside || start_s >= end_s {
        return Err(invalid("tratto non valido: start_s deve precedere end_s e stare nel frame"));
    }
    let i0 = ((start_s - wave.t0) * wave.sample_rate).round() as usize;
    let i1 = (((end_s - wave.t0) * wave.sample_rate).round() as usize + 1).min(n);
    Ok(Waveform {
        samples: wave.samples[i0..i1].to_vec(),
        sample_rate: wave.sample_rate,
        t0: wave.t0 + i0 as f64 / wave.sample_rate,
    })
}

/// Percorso del file esportato: assoluto, oppure relativo a `$HOME/h6022-recordings`.
/// Senza estensione ne aggiunge una; con un'estensione diversa dal formato è errore.
fn export_target(raw: &str, format: ExportFormatName) -> Result<PathBuf, ErrorData> {
    if raw.trim().is_empty() {
        return Err(invalid("percorso vuoto"));
    }
    let ext = match format {
        ExportFormatName::Vcd => "vcd",
        ExportFormatName::Sr => "sr",
    };
    let mut path = PathBuf::from(raw);
    match path.extension().and_then(|e| e.to_str()) {
        None => path.set_extension(ext),
        Some(e) if e.eq_ignore_ascii_case(ext) => true,
        Some(_) => return Err(invalid("l'estensione del percorso non corrisponde al formato")),
    };
    if !path.is_absolute() {
        let dir = h6022_record::recordings_dir().map_err(|e| internal(e.to_string()))?;
        path = dir.join(path);
    }
    Ok(path)
}

/// Scrive il file senza sovrascrivere: se esiste già è `invalid_params`. In caso di errore lo rimuove.
fn write_export(
    path: &Path,
    format: ExportFormatName,
    traces: &[(&str, &LogicTrace)],
    annotations: &[Annotation],
) -> Result<u64, ErrorData> {
    let file = OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => invalid(format!("file esistente: {}", path.display())),
        _ => internal(format!("{}: {e}", path.display())),
    })?;
    let mut out = BufWriter::new(file);
    let result = match format {
        ExportFormatName::Vcd => write_vcd(&mut out, traces, annotations),
        ExportFormatName::Sr => write_sigrok(&mut out, traces),
    }
    .and_then(|()| out.flush());
    if let Err(e) = result {
        let _ = std::fs::remove_file(path);
        return Err(match e.kind() {
            ErrorKind::InvalidInput => invalid(e.to_string()),
            _ => internal(e.to_string()),
        });
    }
    std::fs::metadata(path).map(|m| m.len()).map_err(|e| internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use h6022_core::sim::SimulatedSource;
    use h6022_core::{Engine, EngineConfig, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_dsp::measure::measure;
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::*;
    use crate::AppState;

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

    /// MCP con un frame già pubblicato, poi acquisizione ferma: l'ultimo frame non cambia durante il test.
    fn mcp_with_frame() -> H6022Mcp {
        let source = Box::new(SimulatedSource::bench_default());
        let engine = Engine::start(source, test_config()).unwrap();
        let state = AppState {
            engine,
            source: "simulator",
            addr: SocketAddr::from(([127, 0, 0, 1], 6022)),
            recorder: Arc::new(Mutex::new(None)),
        };
        let rx = state.engine.subscribe();
        rx.recv_timeout(Duration::from_secs(5)).expect("nessun frame dal motore simulato");
        state.engine.pause();
        H6022Mcp::new(state)
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn decode_pwm_canale_1_senza_errori_e_duty_50() {
        let mcp = mcp_with_frame();
        let params = DecodeParams { protocol: "pwm".into(), channel: 1, ..Default::default() };
        let view = block_on(mcp.decode(Parameters(params))).unwrap().0;
        assert!(view.annotations.iter().all(|a| !a.error));
        let duties: Vec<u64> = view.annotations.iter().filter_map(|a| a.value).collect();
        assert!(!duties.is_empty(), "nessun ciclo PWM");
        let mean = duties.iter().sum::<u64>() as f64 / duties.len() as f64;
        assert!((450.0..=550.0).contains(&mean), "duty medio {mean} permille");
    }

    #[test]
    fn classify_canale_1_e_clock() {
        let mcp = mcp_with_frame();
        let view = block_on(mcp.classify_signal(Parameters(ChannelParams { channel: 1 }))).unwrap().0;
        assert_eq!(view.class, "clock");
    }

    #[test]
    fn find_packets_non_va_in_errore() {
        let mcp = mcp_with_frame();
        let params = SegmentsParams { channel: 1, min_gap_s: None };
        assert!(block_on(mcp.find_packets(Parameters(params))).is_ok());
    }

    #[test]
    fn find_pattern_trova_due_periodi_del_canale_1() {
        let mcp = mcp_with_frame();
        let frame = mcp.latest().unwrap();
        let wave = &frame.ch1;
        let period = measure(wave).period.expect("periodo del canale 1");
        let params = PatternParams {
            channel: 1,
            start_s: wave.t0,
            end_s: wave.t0 + period,
            min_score: None,
            max_matches: None,
        };
        let view = block_on(mcp.find_pattern(Parameters(params))).unwrap().0;
        let strong = view.matches.iter().filter(|m| m.score > 0.9).count();
        assert!(strong >= 2, "match forti: {strong}");
    }

    #[test]
    fn protocollo_sconosciuto_e_errore() {
        let mcp = mcp_with_frame();
        let params = DecodeParams { protocol: "xyz".into(), channel: 1, ..Default::default() };
        assert!(block_on(mcp.decode(Parameters(params))).is_err());
    }

    #[test]
    fn export_vcd_inizia_con_date() {
        let mcp = mcp_with_frame();
        let dir = std::env::temp_dir().join(format!("h6022-decode-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.vcd");
        let _ = std::fs::remove_file(&path);
        let params = ExportParams { path: path.to_string_lossy().into_owned(), format: ExportFormatName::Vcd, decode: None };
        let view = block_on(mcp.export_logic(Parameters(params))).unwrap().0;
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("$date"));
        assert_eq!(view.bytes, text.len() as u64);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
