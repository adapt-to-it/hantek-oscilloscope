//! Riga di comando `h6022` per l'oscilloscopio Hantek 6022BE.

mod parse;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use h6022_usb::{
    Block, Channel, ChannelMode, Coupling, Hantek6022, SampleRate, StreamConfig, StreamStats,
    VoltageRange,
};

use parse::{parse_channels, parse_range, parse_rate, parse_u16};

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

/// Legge blocchi finché ogni canale attivo ha almeno `samples` campioni, poi tronca.
fn acquire(dev: &mut Hantek6022, dual: bool, samples: usize) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut stream = dev.start_stream(StreamConfig::default())?;
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
    let stats = measured?;
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
    Ok(())
}

/// Riceve blocchi per la durata indicata e restituisce le statistiche dello stream.
fn measure(dev: &mut Hantek6022, limit: Duration) -> Result<StreamStats> {
    let mut stream = dev.start_stream(StreamConfig::default())?;
    let mut block = Block::default();
    let started = Instant::now();
    while started.elapsed() < limit {
        block.ch1.clear();
        block.ch2.clear();
        stream.next_block(&mut block)?;
    }
    Ok(stream.stats())
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
