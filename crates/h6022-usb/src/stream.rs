//! Streaming continuo dei campioni.

use std::time::{Duration, Instant};

use crate::Result;
use crate::transport::BulkSource;
use crate::types::{ChannelMode, SampleRate};

/// Durata di un trasferimento bulk con `StreamConfig::for_rate`.
const TRANSFER_DURATION: Duration = Duration::from_millis(40);
/// Pacchetto bulk High Speed: il firmware invia solo pacchetti pieni.
const PACKET_SIZE: usize = 512;
const MAX_TRANSFER_SIZE: usize = 256 * 1024;
/// Firma del pacchetto marcatore: il firmware lo invia quando il GPIF riparte dopo una FIFO piena.
const SEGMENT_MAGIC: &[u8; 8] = b"H6022SEG";

#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    /// Byte per trasferimento bulk.
    pub transfer_size: usize,
    /// Trasferimenti in coda contemporaneamente.
    pub queue_depth: usize,
    /// Timeout di ogni lettura.
    pub timeout: Duration,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            transfer_size: 256 * 1024,
            queue_depth: 8,
            timeout: Duration::from_secs(1),
        }
    }
}

impl StreamConfig {
    /// Trasferimenti di circa `TRANSFER_DURATION` alla frequenza indicata.
    ///
    /// Il firmware non chiude i trasferimenti con pacchetti corti: a frequenze basse un
    /// trasferimento da 256 KB dura più del timeout. Trasferimenti più piccoli evitano
    /// il timeout e riducono la latenza. Sopra 6,5 MB/s resta il massimo di 256 KB.
    pub fn for_rate(rate: SampleRate, mode: ChannelMode) -> Self {
        let bytes = rate.bytes_per_second(mode) as f64 * TRANSFER_DURATION.as_secs_f64();
        let packets = (bytes / PACKET_SIZE as f64).ceil().max(1.0) as usize;
        StreamConfig {
            transfer_size: (packets * PACKET_SIZE).min(MAX_TRANSFER_SIZE),
            ..StreamConfig::default()
        }
    }
}

/// Blocco di campioni grezzi separati per canale.
#[derive(Debug, Clone, Default)]
pub struct Block {
    /// Indice del primo campione del blocco dall'inizio dello stream.
    pub first_sample: u64,
    pub ch1: Vec<u8>,
    /// Vuoto in modalità `Single`.
    pub ch2: Vec<u8>,
    /// Indici in `ch1` e `ch2` dove inizia un nuovo segmento, in ordine crescente.
    /// Tra l'ultimo campione prima dell'indice e il primo dopo mancano campioni.
    pub breaks: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamStats {
    pub bytes: u64,
    pub transfers: u64,
    /// Interruzioni dell'acquisizione per FIFO piena. Ogni segmento è continuo.
    pub segments: u64,
    pub elapsed: Duration,
}

impl StreamStats {
    pub fn megabytes_per_second(&self) -> f64 {
        if self.elapsed.is_zero() {
            return 0.0;
        }
        self.bytes as f64 / 1e6 / self.elapsed.as_secs_f64()
    }
}

pub struct Stream<B: BulkSource> {
    source: B,
    mode: ChannelMode,
    raw: Vec<u8>,
    /// Byte di CH1 rimasto senza il suo CH2 alla fine del trasferimento precedente.
    carry: Option<u8>,
    /// Campioni da scartare all'inizio di ogni segmento.
    segment_head: usize,
    /// Campioni ancora da scartare nel segmento corrente.
    skip: usize,
    samples: u64,
    stats: StreamStats,
    started: Instant,
    timeout: Duration,
}

impl<B: BulkSource> Stream<B> {
    /// `segment_head`: campioni non validi all'inizio di ogni segmento.
    pub fn new(source: B, mode: ChannelMode, segment_head: usize, timeout: Duration) -> Self {
        Stream {
            source,
            mode,
            raw: Vec::new(),
            carry: None,
            segment_head,
            // Anche il primo segmento parte da un ADC fermo.
            skip: segment_head,
            samples: 0,
            stats: StreamStats::default(),
            started: Instant::now(),
            timeout,
        }
    }

    /// Legge un trasferimento e lo separa per canale dentro `block`, riusandone i buffer.
    /// I pacchetti marcatore non entrano nei campioni: diventano indici in `block.breaks`.
    pub fn next_block(&mut self, block: &mut Block) -> Result<()> {
        self.raw.clear();
        let n = self.source.read_into(&mut self.raw, self.timeout)?;
        self.stats.bytes += n as u64;
        self.stats.transfers += 1;

        block.ch1.clear();
        block.ch2.clear();
        block.breaks.clear();
        block.first_sample = self.samples;
        let raw = std::mem::take(&mut self.raw);
        let mut data_start = 0;
        for at in (0..raw.len()).step_by(PACKET_SIZE) {
            if raw[at..].len() >= PACKET_SIZE && raw[at..].starts_with(SEGMENT_MAGIC) {
                self.push_samples(block, &raw[data_start..at]);
                self.begin_segment(block);
                data_start = at + PACKET_SIZE;
            }
        }
        self.push_samples(block, &raw[data_start..]);
        self.raw = raw;
        self.samples += block.ch1.len() as u64;
        Ok(())
    }

    fn begin_segment(&mut self, block: &mut Block) {
        self.stats.segments += 1;
        // Il segmento precedente finisce con pacchetti pieni: nessun CH1 resta in sospeso.
        self.carry = None;
        self.skip = self.segment_head;
        let at = block.ch1.len();
        if block.breaks.last() != Some(&at) {
            block.breaks.push(at);
        }
    }

    fn push_samples(&mut self, block: &mut Block, raw: &[u8]) {
        let width = match self.mode {
            ChannelMode::Single => 1,
            ChannelMode::Dual => 2,
        };
        let drop = (self.skip * width).min(raw.len() / width * width);
        self.skip -= drop / width;
        let raw = &raw[drop..];
        match self.mode {
            ChannelMode::Single => block.ch1.extend_from_slice(raw),
            ChannelMode::Dual => {
                // Coppie CH1, CH2 interleaved; un CH1 senza CH2 passa al trasferimento successivo.
                let mut raw = raw;
                if let Some(ch1) = self.carry.take() {
                    match raw.split_first() {
                        Some((&ch2, rest)) => {
                            block.ch1.push(ch1);
                            block.ch2.push(ch2);
                            raw = rest;
                        }
                        None => self.carry = Some(ch1),
                    }
                }
                let (pairs, remainder) = raw.as_chunks::<2>();
                self.carry = self.carry.or(remainder.first().copied());
                block.ch1.reserve(pairs.len());
                block.ch2.reserve(pairs.len());
                for &[ch1, ch2] in pairs {
                    block.ch1.push(ch1);
                    block.ch2.push(ch2);
                }
            }
        }
    }

    pub fn stats(&self) -> StreamStats {
        StreamStats {
            elapsed: self.started.elapsed(),
            ..self.stats
        }
    }

    pub fn mode(&self) -> ChannelMode {
        self.mode
    }
}
