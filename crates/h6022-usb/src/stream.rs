//! Streaming continuo dei campioni.

use std::time::{Duration, Instant};

use crate::Result;
use crate::transport::BulkSource;
use crate::types::ChannelMode;

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

/// Blocco di campioni grezzi separati per canale.
#[derive(Debug, Clone, Default)]
pub struct Block {
    /// Indice del primo campione del blocco dall'inizio dello stream.
    pub first_sample: u64,
    pub ch1: Vec<u8>,
    /// Vuoto in modalità `Single`.
    pub ch2: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamStats {
    pub bytes: u64,
    pub transfers: u64,
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
    samples: u64,
    stats: StreamStats,
    started: Instant,
    timeout: Duration,
}

impl<B: BulkSource> Stream<B> {
    pub fn new(source: B, mode: ChannelMode, timeout: Duration) -> Self {
        Stream {
            source,
            mode,
            raw: Vec::new(),
            carry: None,
            samples: 0,
            stats: StreamStats::default(),
            started: Instant::now(),
            timeout,
        }
    }

    /// Legge un trasferimento e lo separa per canale dentro `block`, riusandone i buffer.
    pub fn next_block(&mut self, block: &mut Block) -> Result<()> {
        self.raw.clear();
        let n = self.source.read_into(&mut self.raw, self.timeout)?;
        self.stats.bytes += n as u64;
        self.stats.transfers += 1;

        block.ch1.clear();
        block.ch2.clear();
        block.first_sample = self.samples;
        match self.mode {
            ChannelMode::Single => block.ch1.extend_from_slice(&self.raw),
            ChannelMode::Dual => {
                // Coppie CH1, CH2 interleaved; un CH1 senza CH2 passa al trasferimento successivo.
                let mut raw = &self.raw[..];
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
        self.samples += block.ch1.len() as u64;
        Ok(())
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
