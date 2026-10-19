//! Riga di comando `h6022` per l'oscilloscopio Hantek 6022BE.

mod parse;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use h6022_core::hardware::HardwareSource;
use h6022_core::sim::SimulatedSource;
use h6022_core::{
    Engine, EngineConfig, EngineHandle, Frame, SampleSource, Slope, TriggerConfig, TriggerKind,
    TriggerMode,
};
use h6022_dsp::{Measurements, Waveform, Window};
use h6022_usb::{
    AcquisitionConfig, Block, Channel, ChannelMode, Coupling, Hantek6022, SampleRate,
    StreamConfig, StreamStats, VoltageRange, fault,
};

use parse::{
    parse_channels, parse_mode, parse_range, parse_rate, parse_trigger_source, parse_u16,
};

#[derive(Parser)]
#[command(
    name = "h6022",
    version,
    about = "Riga di comando per l'oscilloscopio Hantek 6022BE"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mostra versione del firmware, configurazione e calibrazione.
    Info,
    /// Acquisisce campioni e li salva in CSV (in volt) o in byte grezzi con --raw.
    Capture {
        /// Frequenza di campionamento, per esempio 48M, 500k, 20000.
        #[arg(long, value_parser = parse_rate)]
        rate: SampleRate,
        /// Numero di canali acquisiti: 1 o 2.
        #[arg(long, value_parser = parse_channels)]
        channels: ChannelMode,
        /// Portata del canale 1 in volt: 5, 2.5, 1, 0.5.
        #[arg(long, value_parser = parse_range)]
        range1: VoltageRange,
        /// Portata del canale 2 in volt: 5, 2.5, 1, 0.5.
        #[arg(long, value_parser = parse_range)]
        range2: VoltageRange,
        /// Campioni per canale.
        #[arg(long)]
        samples: usize,
        /// File di uscita.
        #[arg(long)]
        out: PathBuf,
        /// Scrive i byte grezzi interleaved invece del CSV.
        #[arg(long)]
        raw: bool,
    },
    /// Misura il throughput dello streaming per un numero di secondi.
    Bench {
        /// Frequenza di campionamento, per esempio 48M, 500k, 20000.
        #[arg(long, value_parser = parse_rate)]
        rate: SampleRate,
        /// Numero di canali acquisiti: 1 o 2.
        #[arg(long, value_parser = parse_channels)]
        channels: ChannelMode,
        /// Durata della misura in secondi.
        #[arg(long)]
        seconds: f64,
    },
    /// Stampa un tratto della EEPROM in esadecimale.
    Eeprom {
        /// Offset di partenza, decimale o con prefisso 0x.
        #[arg(long, value_parser = parse_u16)]
        offset: u16,
        /// Numero di byte da leggere, decimale o con prefisso 0x.
        #[arg(long, value_parser = parse_u16)]
        len: u16,
    },
    /// Misura tensione, frequenza e spettro su più frame sincronizzati.
    Measure(MeasureArgs),
    /// Avvia il server MCP e WebSocket senza interfaccia grafica.
    Serve {
        /// Porta su 127.0.0.1.
        #[arg(long, default_value_t = h6022_server::DEFAULT_PORT)]
        port: u16,
        /// Usa il simulatore invece del dispositivo.
        #[arg(long)]
        sim: bool,
    },
}

#[derive(Args)]
struct MeasureArgs {
    /// Frequenza di campionamento, per esempio 48M, 500k, 20000.
    #[arg(long, value_parser = parse_rate)]
    rate: SampleRate,
    /// Numero di canali acquisiti: 1 o 2.
    #[arg(long, value_parser = parse_channels)]
    channels: ChannelMode,
    /// Portata del canale 1 in volt: 5, 2.5, 1, 0.5.
    #[arg(long, value_parser = parse_range)]
    range1: VoltageRange,
    /// Portata del canale 2 in volt: 5, 2.5, 1, 0.5.
    #[arg(long, value_parser = parse_range)]
    range2: VoltageRange,
    /// Campioni per canale in ogni frame.
    #[arg(long, default_value_t = 100_000)]
    record: usize,
    /// Numero di frame da raccogliere e mediare.
    #[arg(long, default_value_t = 5)]
    frames: usize,
    /// Livello del trigger in volt.
    #[arg(long, default_value_t = 1.5, allow_negative_numbers = true)]
    trigger_level: f32,
    /// Sorgente del trigger: 1 o 2.
    #[arg(long, value_parser = parse_trigger_source)]
    trigger_source: Channel,
    /// Modalità del trigger: auto o normal.
    #[arg(long, value_parser = parse_mode)]
    mode: TriggerMode,
    /// Usa il simulatore invece del dispositivo.
    #[arg(long)]
    sim: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info => info(),
        Command::Capture {
            rate,
            channels,
            range1,
            range2,
            samples,
            out,
            raw,
        } => capture(rate, channels, [range1, range2], samples, &out, raw),
        Command::Bench {
            rate,
            channels,
            seconds,
        } => bench(rate, channels, seconds),
        Command::Eeprom { offset, len } => eeprom(offset, len),
        Command::Measure(args) => run_measure(&args),
        Command::Serve { port, sim } => serve(port, sim),
    }
}

/// Apre il dispositivo e applica frequenza e numero di canali.
fn open_with(rate: SampleRate, mode: ChannelMode) -> Result<Hantek6022> {
    let mut dev = Hantek6022::open().context("apertura dell'Hantek 6022BE")?;
    dev.set_sample_rate(rate)?;
    dev.set_channel_mode(mode)?;
    Ok(dev)
}

fn info() -> Result<()> {
    let dev = Hantek6022::open().context("apertura dell'Hantek 6022BE")?;
    println!("Firmware: 0x{:04X}", dev.firmware_version());
    if let Some(info) = dev.firmware_info()? {
        println!("Errore del firmware: {}", fault::describe(info.fault));
    }

    let cfg = dev.config();
    println!("Frequenza di campionamento: {}", cfg.sample_rate.label());
    println!("Canali: {}", mode_label(cfg.mode));
    for (i, range) in cfg.range.iter().enumerate() {
        println!(
            "CH{}: portata {}, accoppiamento {}",
            i + 1,
            range_label(*range),
            coupling_label(cfg.coupling[i])
        );
    }

    let cal = dev.calibration();
    println!();
    println!("Calibrazione (offset in conteggi ADC, gain):");
    println!("{:<8}{:<12}{:>12}{:>12}", "canale", "portata", "offset", "gain");
    for (i, channel) in [Channel::Ch1, Channel::Ch2].into_iter().enumerate() {
        for range in VoltageRange::all() {
            println!(
                "{:<8}{:<12}{:>12.3}{:>12.4}",
                format!("CH{}", i + 1),
                range_label(range),
                cal.offset(channel, range),
                cal.gain(channel, range)
            );
        }
    }
    Ok(())
}

fn capture(
    rate: SampleRate,
    mode: ChannelMode,
    ranges: [VoltageRange; 2],
    samples: usize,
    out: &Path,
    raw: bool,
) -> Result<()> {
    if samples == 0 {
        bail!("--samples deve essere maggiore di zero");
    }
    let dual = mode == ChannelMode::Dual;

    let mut dev = open_with(rate, mode)?;
    dev.set_voltage_range(Channel::Ch1, ranges[0])?;
    dev.set_voltage_range(Channel::Ch2, ranges[1])?;
    let acquired = acquire(&mut dev, dual, samples);
    let stopped = dev.stop();
    let (ch1, ch2) = acquired?;
    stopped?;

    let file = File::create(out).with_context(|| format!("creazione di {}", out.display()))?;
    let mut w = BufWriter::new(file);
    if raw {
        if dual {
            for (a, b) in ch1.iter().zip(ch2.iter()) {
                w.write_all(&[*a, *b])?;
            }
        } else {
            w.write_all(&ch1)?;
        }
    } else {
        let cal = dev.calibration();
        let mut v1 = Vec::with_capacity(samples);
        cal.to_volts_slice(Channel::Ch1, ranges[0], &ch1, &mut v1);
        let mut v2 = Vec::with_capacity(samples);
        if dual {
            cal.to_volts_slice(Channel::Ch2, ranges[1], &ch2, &mut v2);
        }

        let rate_hz = f64::from(rate.hz());
        writeln!(w, "t_s,ch1_v{}", if dual { ",ch2_v" } else { "" })?;
        for (i, a) in v1.iter().enumerate() {
            let t = i as f64 / rate_hz;
            write!(w, "{t:.9},{a:.6}")?;
            if dual {
                write!(w, ",{:.6}", v2[i])?;
            }
            writeln!(w)?;
        }
    }
    w.flush()?;

    println!("Scritti {samples} campioni per canale in {}", out.display());
    Ok(())
}

/// Trasferimenti dimensionati sulla frequenza e sui canali correnti del dispositivo.
fn stream_config(dev: &Hantek6022) -> StreamConfig {
    let config = dev.config();
    StreamConfig::for_rate(config.sample_rate, config.mode)
}

/// Legge blocchi finché ogni canale attivo ha almeno `samples` campioni, poi tronca.
fn acquire(dev: &mut Hantek6022, dual: bool, samples: usize) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut stream = dev.start_stream(stream_config(dev))?;
    let mut block = Block::default();
    let mut ch1 = Vec::with_capacity(samples);
    let mut ch2 = Vec::with_capacity(if dual { samples } else { 0 });
    while ch1.len() < samples || (dual && ch2.len() < samples) {
        block.ch1.clear();
        block.ch2.clear();
        stream.next_block(&mut block)?;
        ch1.extend_from_slice(&block.ch1);
        ch2.extend_from_slice(&block.ch2);
    }
    ch1.truncate(samples);
    ch2.truncate(samples);
    Ok((ch1, ch2))
}

fn bench(rate: SampleRate, mode: ChannelMode, seconds: f64) -> Result<()> {
    let limit = match Duration::try_from_secs_f64(seconds) {
        Ok(d) if !d.is_zero() => d,
        _ => bail!("--seconds deve essere un numero positivo"),
    };

    let mut dev = open_with(rate, mode)?;
    let measured = measure(&mut dev, limit);
    let stopped = dev.stop();
    let (stats, overflows) = measured?;
    stopped?;

    let measured_mbs = stats.megabytes_per_second();
    let expected_mbs = rate.bytes_per_second(mode) as f64 / 1e6;
    println!("Frequenza di campionamento: {}", rate.label());
    println!("Canali: {}", mode_label(mode));
    println!(
        "Byte ricevuti: {}, trasferimenti: {}, durata: {:.2} s",
        stats.bytes,
        stats.transfers,
        stats.elapsed.as_secs_f64()
    );
    println!("MB/s misurati: {measured_mbs:.2}");
    println!("MB/s attesi: {expected_mbs:.2}");
    println!("Rapporto: {:.1} %", measured_mbs / expected_mbs * 100.0);
    match overflows {
        Some(0) => println!("FIFO piena: mai, flusso continuo"),
        Some(n) => println!(
            "FIFO piena: {n} volte (contatore modulo 65536); segmenti ricevuti: {}, ognuno senza perdite",
            stats.segments
        ),
        None => println!("FIFO piena: non rilevabile con il firmware {:#06x}", dev.firmware_version()),
    }
    if let Some(info) = dev.firmware_info()?
        && info.fault != fault::NONE
    {
        println!("Errore del firmware: {}", fault::describe(info.fault));
    }
    Ok(())
}

/// Riceve blocchi per la durata indicata e restituisce le statistiche dello stream
/// e il contatore di FIFO piena.
fn measure(dev: &mut Hantek6022, limit: Duration) -> Result<(StreamStats, Option<u16>)> {
    let mut stream = dev.start_stream(stream_config(dev))?;
    let mut block = Block::default();
    let started = Instant::now();
    while started.elapsed() < limit {
        block.ch1.clear();
        block.ch2.clear();
        stream.next_block(&mut block)?;
    }
    // Letto con lo stream ancora aperto: dopo la chiusura della coda la FIFO si riempie.
    let overflows = dev.overflow_count()?;
    Ok((stream.stats(), overflows))
}

/// Timeout massimo per ricevere un frame dal motore.
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);
/// Portata USB oltre la quale il firmware divide il flusso in segmenti continui.
const USB_BYTES_PER_SECOND: f64 = 45e6;
/// Isteresi del trigger in volt.
const TRIGGER_HYSTERESIS: f32 = 0.1;
/// Frazione del frame prima del trigger.
const PRETRIGGER: f32 = 0.5;

/// Avvia motore e server, poi resta in attesa finché il processo non viene interrotto.
fn serve(port: u16, sim: bool) -> Result<()> {
    let (source, name): (Box<dyn SampleSource>, &'static str) = if sim {
        (Box::new(SimulatedSource::bench_default()), "simulator")
    } else {
        (Box::new(HardwareSource::open().context("apertura dell'Hantek 6022BE")?), "hardware")
    };
    let config = EngineConfig {
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
            hysteresis: TRIGGER_HYSTERESIS,
            holdoff: 0.0,
        },
        mode: TriggerMode::Auto,
        record_length: 10_000,
        pretrigger: PRETRIGGER,
        probe: [1.0, 1.0],
    };
    let engine = Engine::start(source, config)?;
    let server_config = h6022_server::ServerConfig {
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
    };
    let server = h6022_server::spawn(engine.clone(), name, server_config)?;
    let addr = server.addr();
    println!("MCP:       http://{addr}/mcp");
    println!("WebSocket: ws://{addr}/ws");
    println!("Per Claude Code: claude mcp add --transport http h6022 http://{addr}/mcp");
    // Con SIGINT o SIGTERM ferma server e streaming: un processo terminato a metà
    // di uno streaming lascia il firmware senza risposta.
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })?;
    let _ = rx.recv();
    server.shutdown();
    engine.shutdown()?;
    Ok(())
}

/// Raccoglie `frames` frame dal motore e stampa le misure medie per canale.
fn run_measure(args: &MeasureArgs) -> Result<()> {
    if args.frames == 0 {
        bail!("--frames deve essere maggiore di zero");
    }
    let dual = args.channels == ChannelMode::Dual;
    let config = EngineConfig {
        acquisition: AcquisitionConfig {
            sample_rate: args.rate,
            mode: args.channels,
            range: [args.range1, args.range2],
            coupling: [Coupling::Dc, Coupling::Dc],
        },
        trigger: TriggerConfig {
            source: args.trigger_source,
            kind: TriggerKind::Edge {
                slope: Slope::Rising,
            },
            level: args.trigger_level,
            hysteresis: TRIGGER_HYSTERESIS,
            holdoff: 0.0,
        },
        mode: args.mode,
        record_length: args.record,
        pretrigger: PRETRIGGER,
        probe: [1.0, 1.0],
    };

    let source: Box<dyn SampleSource> = if args.sim {
        Box::new(SimulatedSource::bench_default())
    } else {
        Box::new(HardwareSource::open().context("apertura dell'Hantek 6022BE")?)
    };
    let engine = Engine::start(source, config)?;
    let collected = collect_frames(&engine, args.frames).map_err(|err| {
        let channels = if dual { 2.0 } else { 1.0 };
        if f64::from(args.rate.hz()) * channels > USB_BYTES_PER_SECOND {
            // Il motore non costruisce un record a cavallo di due segmenti.
            err.context(format!(
                "a {} con {} il flusso supera i 45 MB/s dell'USB e arriva in segmenti \
                 continui di poche migliaia di campioni: riduci --record, per esempio a 2000",
                args.rate.label(),
                if dual { "due canali" } else { "un canale" }
            ))
        } else {
            err
        }
    });
    let stopped = engine.shutdown();
    let frames = collected?;
    stopped?;

    print_measure(&frames, dual);
    Ok(())
}

/// Riceve `count` frame dal motore, con un timeout per frame.
fn collect_frames(engine: &EngineHandle, count: usize) -> Result<Vec<Arc<Frame>>> {
    let rx = engine.subscribe();
    let mut frames = Vec::new();
    while frames.len() < count {
        match rx.recv_timeout(FRAME_TIMEOUT) {
            Ok(frame) => frames.push(frame),
            Err(RecvTimeoutError::Timeout) => bail!(
                "nessun frame dal motore entro {} s",
                FRAME_TIMEOUT.as_secs()
            ),
            Err(RecvTimeoutError::Disconnected) => {
                bail!("il motore si è fermato durante la misura")
            }
        }
    }
    Ok(frames)
}

/// Stampa i frame sincronizzati, le medie per canale e i picchi dello spettro dell'ultimo frame.
fn print_measure(frames: &[Arc<Frame>], dual: bool) {
    let synced = frames.iter().filter(|f| f.triggered).count();
    println!("Frame sincronizzati: {synced} su {}", frames.len());

    let last = frames.last();
    let channels = if dual { 2 } else { 1 };
    for index in 0..channels {
        let waves: Vec<&Waveform> = frames
            .iter()
            .filter_map(|f| channel_wave(f, index))
            .collect();
        let ms: Vec<Measurements> = waves.into_iter().map(h6022_dsp::measure).collect();

        println!("CH{} (media su {} frame)", index + 1, ms.len());
        let vpp = mean(&ms, |m| Some(f64::from(m.vpp)));
        let vtop = mean(&ms, |m| Some(f64::from(m.vtop)));
        let vbase = mean(&ms, |m| Some(f64::from(m.vbase)));
        let vrms = mean(&ms, |m| Some(f64::from(m.vrms)));
        let freq = mean(&ms, |m| m.frequency);
        let duty = mean(&ms, |m| m.duty_positive).map(|d| d * 100.0);
        let rise = mean(&ms, |m| m.rise_time).map(|t| t * 1e6);
        let fall = mean(&ms, |m| m.fall_time).map(|t| t * 1e6);
        println!("  vpp        {}", fmt_value(vpp, 3, "V"));
        println!("  vtop       {}", fmt_value(vtop, 3, "V"));
        println!("  vbase      {}", fmt_value(vbase, 3, "V"));
        println!("  vrms       {}", fmt_value(vrms, 3, "V"));
        println!("  frequenza  {}", fmt_value(freq, 2, "Hz"));
        println!("  duty       {}", fmt_value(duty, 1, "%"));
        println!("  salita     {}", fmt_value(rise, 3, "µs"));
        println!("  discesa    {}", fmt_value(fall, 3, "µs"));

        if let Some(wave) = last.and_then(|f| channel_wave(f, index)) {
            let spec = h6022_dsp::spectrum::spectrum(wave, Window::FlatTop);
            println!("  picchi spettro FlatTop (ultimo frame):");
            let peaks = spec.peaks(5);
            if peaks.is_empty() {
                println!("    —");
            }
            for p in peaks {
                println!("    {:>12.1} Hz  {:.4} V", p.frequency, p.vrms);
            }
            let thd = freq.and_then(|f| spec.thd_pct(f));
            println!("  THD        {}", fmt_value(thd, 3, "%"));
        }
    }
}

/// Forma d'onda del canale `index` (0 = CH1, 1 = CH2), se presente nel frame.
fn channel_wave(frame: &Frame, index: usize) -> Option<&Waveform> {
    match index {
        0 => Some(&frame.ch1),
        _ => frame.ch2.as_ref(),
    }
}

/// Media dei valori presenti; `None` se nessuna misura ha il valore.
fn mean<F>(ms: &[Measurements], get: F) -> Option<f64>
where
    F: Fn(&Measurements) -> Option<f64>,
{
    let values: Vec<f64> = ms.iter().filter_map(get).collect();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

/// Valore con decimali e unità, oppure "—" se assente.
fn fmt_value(value: Option<f64>, decimals: usize, unit: &str) -> String {
    match value {
        Some(v) => format!("{v:.decimals$} {unit}"),
        None => "—".to_string(),
    }
}

fn eeprom(offset: u16, len: u16) -> Result<()> {
    if len == 0 {
        bail!("--len deve essere maggiore di zero");
    }
    let dev = Hantek6022::open().context("apertura dell'Hantek 6022BE")?;
    let bytes = dev.read_eeprom(offset, len)?;
    for (i, row) in bytes.chunks(16).enumerate() {
        let addr = usize::from(offset) + i * 16;
        let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
        println!("{addr:04x}: {}", hex.join(" "));
    }
    Ok(())
}

fn mode_label(mode: ChannelMode) -> &'static str {
    match mode {
        ChannelMode::Single => "1",
        ChannelMode::Dual => "2",
    }
}

fn coupling_label(coupling: Coupling) -> &'static str {
    match coupling {
        Coupling::Ac => "AC",
        Coupling::Dc => "DC",
    }
}

fn range_label(range: VoltageRange) -> &'static str {
    match range {
        VoltageRange::V5 => "±5 V",
        VoltageRange::V2_5 => "±2,5 V",
        VoltageRange::V1 => "±1 V",
        VoltageRange::V0_5 => "±0,5 V",
    }
}
