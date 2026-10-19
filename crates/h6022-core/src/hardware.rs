//! `SampleSource` sul dispositivo reale.

use h6022_usb::nusb_transport::{NusbBulk, NusbTransport};
use h6022_usb::{AcquisitionConfig, Block, Calibration, Channel, Hantek6022, Stream, StreamConfig};

use crate::source::{SampleSource, VoltBlock};
use crate::types::{CoreError, CoreResult};

pub struct HardwareSource {
    device: Hantek6022<NusbTransport>,
    stream: Option<Stream<NusbBulk>>,
    raw: Block,
}

impl HardwareSource {
    /// Apre il dispositivo (carica il firmware se serve).
    pub fn open() -> CoreResult<HardwareSource> {
        Ok(HardwareSource {
            device: Hantek6022::open()?,
            stream: None,
            raw: Block::default(),
        })
    }

    pub fn firmware_version(&self) -> u16 {
        self.device.firmware_version()
    }
}

impl SampleSource for HardwareSource {
    fn configure(&mut self, config: &AcquisitionConfig) -> CoreResult<()> {
        // Chiude la coda precedente prima di riconfigurare il dispositivo.
        self.stream = None;
        self.device.stop()?;
        self.device.set_sample_rate(config.sample_rate)?;
        self.device.set_channel_mode(config.mode)?;
        self.device.set_voltage_range(Channel::Ch1, config.range[0])?;
        self.device.set_voltage_range(Channel::Ch2, config.range[1])?;
        self.device.set_coupling(Channel::Ch1, config.coupling[0])?;
        self.device.set_coupling(Channel::Ch2, config.coupling[1])?;
        let stream_config = StreamConfig::for_rate(config.sample_rate, config.mode);
        self.stream = Some(self.device.start_stream(stream_config)?);
        Ok(())
    }

    fn read(&mut self, out: &mut VoltBlock) -> CoreResult<()> {
        // Senza stream il dispositivo non sta acquisendo: non c'è configurazione attiva.
        let stream = self.stream.as_mut().ok_or(CoreError::Stopped)?;
        stream.next_block(&mut self.raw)?;

        let config = self.device.config();
        let calibration = self.device.calibration();
        out.first_sample = self.raw.first_sample;
        calibration.to_volts_slice(Channel::Ch1, config.range[0], &self.raw.ch1, &mut out.ch1);
        calibration.to_volts_slice(Channel::Ch2, config.range[1], &self.raw.ch2, &mut out.ch2);
        Ok(())
    }

    fn calibration(&self) -> Calibration {
        self.device.calibration().clone()
    }

    fn stop(&mut self) -> CoreResult<()> {
        self.stream = None;
        self.device.stop()?;
        Ok(())
    }
}

/// Richiedono l'Hantek collegato: `cargo test -p h6022-core --lib -- --ignored device_tests`.
#[cfg(test)]
mod device_tests {
    use std::time::{Duration, Instant};

    use h6022_usb::{ChannelMode, Coupling, SampleRate, VoltageRange};

    use super::*;

    #[test]
    #[ignore]
    fn hardware_reads_blocks_at_slow_rates() {
        let mut source = HardwareSource::open().expect("apertura");
        for rate in [SampleRate(102), SampleRate(110)] {
            source
                .configure(&AcquisitionConfig {
                    sample_rate: rate,
                    mode: ChannelMode::Dual,
                    range: [VoltageRange::V5; 2],
                    coupling: [Coupling::Dc; 2],
                })
                .expect("configure");
            let mut block = VoltBlock::default();
            let started = Instant::now();
            source.read(&mut block).expect("read");
            assert!(!block.ch1.is_empty() && block.ch1.len() == block.ch2.len());
            // Un trasferimento dura circa 40 ms: il primo blocco arriva ben prima del timeout.
            assert!(started.elapsed() < Duration::from_millis(500), "{rate:?}: {:?}", started.elapsed());
        }
        source.stop().expect("stop");
    }
}
