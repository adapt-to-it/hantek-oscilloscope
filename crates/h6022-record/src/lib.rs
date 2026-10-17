//! Registrazione su disco dei frame del motore.
//!
//! Ogni formato scrive i frame uno dopo l'altro, nello stesso ordine in cui arrivano.
//! Accanto al file principale nasce `<file>.json` con i metadati della registrazione.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use h6022_core::Frame;

pub mod csv;
pub mod npy;
pub mod recorder;
pub mod wav;

pub use recorder::Recorder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `frame,t_s,ch1_v[,ch2_v]`, una riga per campione.
    Csv,
    /// Matrice `<f4` di forma (campioni totali, canali).
    Npy,
    /// Float 32 bit IEEE, un canale WAV per canale dell'oscilloscopio, frequenza = campionamento.
    Wav,
}

impl Format {
    /// Dall'estensione del percorso: csv, npy, wav, senza distinzione di maiuscole.
    pub fn from_path(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "csv" => Some(Format::Csv),
            "npy" => Some(Format::Npy),
            "wav" => Some(Format::Wav),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Npy => "npy",
            Format::Wav => "wav",
        }
    }
}

/// Scrittore di un formato. Il primo frame fissa frequenza e numero di canali:
/// un frame successivo diverso dà `io::ErrorKind::InvalidInput` e non scrive nulla.
pub trait FrameWriter: Send {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()>;
    /// Completa le intestazioni, chiude il file e scrive il JSON dei metadati.
    fn finish(self: Box<Self>) -> io::Result<RecordSummary>;
}

/// Crea il file (errore se esiste già) e lo scrittore del formato.
pub fn create(path: &Path, format: Format) -> io::Result<Box<dyn FrameWriter>> {
    let file = File::create_new(path)?;
    let path = path.to_path_buf();
    match format {
        Format::Csv => csv::writer(file, path),
        Format::Npy => npy::writer(file, path),
        Format::Wav => wav::writer(file, path),
    }
}

/// Limiti della registrazione: si ferma al primo raggiunto. Senza limiti continua fino a `stop`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecordLimit {
    pub max_frames: Option<u64>,
    pub max_seconds: Option<f64>,
}

/// Contenuto del JSON dei metadati e risultato di `finish`.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordSummary {
    pub path: PathBuf,
    pub format: Format,
    pub frames: u64,
    pub channels: u8,
    pub sample_rate_hz: f64,
    /// Campioni per canale su tutti i frame.
    pub samples_per_channel: u64,
}

/// Stato di una registrazione in corso o appena finita.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordStatus {
    pub path: PathBuf,
    pub format: Format,
    pub frames: u64,
    pub running: bool,
    /// Errore che ha fermato la registrazione.
    pub error: Option<String>,
}

/// Errore `InvalidInput` con il messaggio dato.
pub(crate) fn invalid_input(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

/// Frequenza di campionamento e numero di canali di un frame.
fn shape(frame: &Frame) -> (f64, u8) {
    let channels = if frame.ch2.is_some() { 2 } else { 1 };
    (frame.ch1.sample_rate, channels)
}

/// Stato comune agli scrittori: parametri fissati dal primo frame e conteggi.
#[derive(Debug, Default)]
pub(crate) struct Tally {
    layout: Option<(f64, u8)>,
    pub frames: u64,
    pub samples_per_channel: u64,
    pub record_length: usize,
}

impl Tally {
    /// Frequenza e canali del frame, se compatibili con quelli già scritti. Non cambia lo stato.
    pub fn check(&self, frame: &Frame) -> io::Result<(f64, u8)> {
        let (rate, channels) = shape(frame);
        if let Some(ch2) = &frame.ch2 {
            if ch2.sample_rate != rate || ch2.samples.len() != frame.ch1.samples.len() {
                return Err(invalid_input("canali con frequenza o numero di campioni diversi"));
            }
        }
        match self.layout {
            Some(layout) if layout != (rate, channels) => Err(invalid_input(format!(
                "frame diverso dal primo: {rate} Hz e {channels} canali, attesi {} Hz e {} canali",
                layout.0, layout.1
            ))),
            _ => Ok((rate, channels)),
        }
    }

    /// Registra un frame già scritto.
    pub fn commit(&mut self, frame: &Frame) {
        self.layout = Some(shape(frame));
        self.frames += 1;
        self.samples_per_channel += frame.ch1.samples.len() as u64;
        self.record_length = frame.config.record_length;
    }

    /// Frequenza e canali; senza frame, 0 Hz e un canale.
    pub fn layout(&self) -> (f64, u8) {
        self.layout.unwrap_or((0.0, 1))
    }

    pub fn summary(&self, path: PathBuf, format: Format) -> RecordSummary {
        let (sample_rate_hz, channels) = self.layout();
        RecordSummary {
            path,
            format,
            frames: self.frames,
            channels,
            sample_rate_hz,
            samples_per_channel: self.samples_per_channel,
        }
    }
}

/// Scrive `<file>.json` accanto al file della registrazione.
pub(crate) fn write_metadata(summary: &RecordSummary, record_length: usize) -> io::Result<()> {
    let mut json_path = summary.path.clone().into_os_string();
    json_path.push(".json");
    let meta = serde_json::json!({
        "format": summary.format.extension(),
        "frames": summary.frames,
        "channels": summary.channels,
        "sample_rate_hz": summary.sample_rate_hz,
        "samples_per_channel": summary.samples_per_channel,
        "record_length": record_length,
    });
    let text = serde_json::to_string_pretty(&meta).map_err(io::Error::other)?;
    std::fs::write(json_path, text + "\n")
}

/// Costruttori di frame e configurazioni per i test dei formati e del Recorder.
#[cfg(test)]
pub(crate) mod support {
    use std::path::PathBuf;

    use h6022_core::{EngineConfig, Frame, Slope, TriggerConfig, TriggerKind, TriggerMode};
    use h6022_dsp::Waveform;
    use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};

    /// Cartella vuota e unica per il test, in `temp_dir`.
    pub fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("h6022-record-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Configurazione valida del motore, a uno o due canali.
    pub fn config(mode: TriggerMode, dual: bool, record_length: usize) -> EngineConfig {
        EngineConfig {
            acquisition: AcquisitionConfig {
                sample_rate: SampleRate(1),
                mode: if dual { ChannelMode::Dual } else { ChannelMode::Single },
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
            mode,
            record_length,
            pretrigger: 0.25,
            probe: [1.0, 1.0],
        }
    }

    /// Frame con i campioni dati; `ch2` a `Some` lo rende a due canali.
    pub fn frame(sequence: u64, rate: f64, t0: f64, ch1: Vec<f32>, ch2: Option<Vec<f32>>) -> Frame {
        let dual = ch2.is_some();
        let record_length = ch1.len();
        Frame {
            sequence,
            ch1: Waveform {
                samples: ch1,
                sample_rate: rate,
                t0,
            },
            ch2: ch2.map(|samples| Waveform {
                samples,
                sample_rate: rate,
                t0,
            }),
            triggered: true,
            trigger_sample: Some(0),
            config: config(TriggerMode::Normal, dual, record_length),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::scratch;

    #[test]
    fn formato_dall_estensione_senza_distinzione_di_maiuscole() {
        assert_eq!(Format::from_path(Path::new("rec.NPY")), Some(Format::Npy));
        assert_eq!(Format::from_path(Path::new("dir/rec.csv")), Some(Format::Csv));
        assert_eq!(Format::from_path(Path::new("rec.Wav")), Some(Format::Wav));
        assert_eq!(Format::from_path(Path::new("rec.txt")), None);
        assert_eq!(Format::from_path(Path::new("rec")), None);
    }

    #[test]
    fn create_rifiuta_file_esistente() {
        let dir = scratch("esistente");
        let path = dir.join("rec.csv");
        std::fs::write(&path, b"").unwrap();
        let err = create(&path, Format::Csv).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn frame_con_frequenza_diversa_da_invalid_input() {
        use crate::support::frame;

        let dir = scratch("frequenza");
        let path = dir.join("rec.csv");
        let mut writer = create(&path, Format::Csv).unwrap();
        writer.write_frame(&frame(0, 1000.0, 0.0, vec![1.0], None)).unwrap();
        let err = writer.write_frame(&frame(1, 2000.0, 0.0, vec![2.0], None)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        let summary = writer.finish().unwrap();
        assert_eq!(summary.frames, 1);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "frame,t_s,ch1_v\n0,0.000000000,1.000000\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn registrazione_vuota_scrive_file_e_json() {
        let dir = scratch("vuota");
        for format in [Format::Csv, Format::Npy, Format::Wav] {
            let path = dir.join(format!("rec.{}", format.extension()));
            let summary = create(&path, format).unwrap().finish().unwrap();
            assert_eq!(summary.frames, 0);
            assert_eq!(summary.samples_per_channel, 0);
            let json = std::fs::read_to_string(format!("{}.json", path.display())).unwrap();
            assert!(json.contains("\"frames\": 0"), "{json}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
