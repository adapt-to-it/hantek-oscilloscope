//! Comandi di alto livello per l'Hantek 6022BE.

use std::thread;
use std::time::{Duration, Instant};

use crate::calibration::Calibration;
use crate::firmware::{self, DSO6022BE_HEX};
use crate::nusb_transport::{self, NusbTransport};
use crate::stream::{Stream, StreamConfig};
use crate::transport::{BulkSource, Transport};
use crate::types::{
    BULK_EP_IN, CALIBRATION_EEPROM_LEN, CALIBRATION_EEPROM_OFFSET, Channel, ChannelMode, Coupling,
    FIRMWARE_VERSION, FirmwareInfo, PID_6022BE, SampleRate, VID_FIRMWARE, VID_NO_FIRMWARE, VoltageRange, req,
};
use crate::{Error, Result};

/// Attesa massima per la rienumerazione dopo il caricamento del firmware.
const RENUMERATION_TIMEOUT: Duration = Duration::from_secs(10);
/// Intervallo tra due interrogazioni del bus durante la rienumerazione.
const RENUMERATION_POLL: Duration = Duration::from_millis(100);

/// Impostazioni correnti dell'acquisizione.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcquisitionConfig {
    pub sample_rate: SampleRate,
    pub mode: ChannelMode,
    pub range: [VoltageRange; 2],
    pub coupling: [Coupling; 2],
}

pub struct Hantek6022<T: Transport = NusbTransport> {
    transport: T,
    firmware_version: u16,
    calibration: Calibration,
    config: AcquisitionConfig,
}

impl Hantek6022<NusbTransport> {
    /// Trova il dispositivo, carica il firmware se manca, attende la rienumerazione
    /// e applica la configurazione predefinita.
    pub fn open() -> Result<Self> {
        Self::open_with_timeout(RENUMERATION_TIMEOUT)
    }

    /// Come `open`, con un limite di attesa per la rienumerazione.
    ///
    /// Un processo terminato a metà di uno streaming può lasciare il firmware senza risposta.
    /// In quel caso `open` prova un reset della porta USB e, se non basta, ricarica il firmware:
    /// la richiesta 0xA0 la gestisce il core del FX2, non il firmware.
    /// Se il dispositivo esegue un'altra versione del firmware, `open` carica quella incorporata.
    pub fn open_with_timeout(renumeration_timeout: Duration) -> Result<Self> {
        if !nusb_transport::list(VID_FIRMWARE, PID_6022BE)?.is_empty() {
            let first = match open_firmware_device() {
                Ok(dev) if dev.firmware_version() == FIRMWARE_VERSION => return Ok(dev),
                Ok(dev) => {
                    drop(dev);
                    return reload_firmware(VID_FIRMWARE, renumeration_timeout);
                }
                Err(e) => e,
            };
            if let Ok(dev) = recover_by_reset() {
                return Ok(dev);
            }
            return reload_firmware(VID_FIRMWARE, renumeration_timeout).map_err(|_| first);
        }

        // Senza firmware: `open` restituisce `NotFound` se manca anche questo dispositivo.
        reload_firmware(VID_NO_FIRMWARE, renumeration_timeout)
    }
}

/// Reset della porta, poi un nuovo tentativo di apertura.
fn recover_by_reset() -> Result<Hantek6022<NusbTransport>> {
    nusb_transport::reset(VID_FIRMWARE, PID_6022BE)?;
    open_after_settle()
}

/// Carica il firmware sul dispositivo con `vendor_id`, attende la rienumerazione e lo apre.
fn reload_firmware(vendor_id: u16, renumeration_timeout: Duration) -> Result<Hantek6022<NusbTransport>> {
    let raw = NusbTransport::open(vendor_id, PID_6022BE)?;
    firmware::upload(&raw, DSO6022BE_HEX)?;
    drop(raw);
    if vendor_id == VID_FIRMWARE {
        // Il dispositivo esce dal bus e rientra con lo stesso VID: senza questa attesa
        // il controllo qui sotto troverebbe ancora la vecchia istanza.
        thread::sleep(RENUMERATION_GONE);
    }
    let deadline = Instant::now() + renumeration_timeout;
    while nusb_transport::list(VID_FIRMWARE, PID_6022BE)?.is_empty() {
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        thread::sleep(RENUMERATION_POLL);
    }
    open_after_settle()
}

/// Il dispositivo compare sul bus prima che macOS e il firmware siano pronti:
/// i primi trasferimenti possono andare in timeout o in stallo.
fn open_after_settle() -> Result<Hantek6022<NusbTransport>> {
    let mut last = Error::Timeout;
    for _ in 0..OPEN_ATTEMPTS {
        thread::sleep(SETTLE_DELAY);
        match open_firmware_device() {
            Ok(dev) => return Ok(dev),
            Err(e) => last = e,
        }
    }
    Err(last)
}

const STALE_QUIET: Duration = Duration::from_millis(30);
const OPEN_ATTEMPTS: usize = 3;
const SETTLE_DELAY: Duration = Duration::from_millis(500);
/// Tempo in cui il FX2 esce dal bus dopo un nuovo caricamento del firmware.
const RENUMERATION_GONE: Duration = Duration::from_millis(1500);

/// Apre il dispositivo con il firmware già caricato e ne legge la versione.
fn open_firmware_device() -> Result<Hantek6022<NusbTransport>> {
    let transport = NusbTransport::open(VID_FIRMWARE, PID_6022BE)?;
    // Ferma uno streaming lasciato attivo da un processo terminato.
    transport.control_out(req::START_STOP, 0, 0, &[0])?;
    let firmware_version = transport.id().device_version;
    Hantek6022::with_transport(transport, firmware_version)
}

impl<T: Transport> Hantek6022<T> {
    /// Usa un trasporto con il firmware già caricato: legge la calibrazione
    /// e applica la configurazione predefinita.
    pub fn with_transport(transport: T, firmware_version: u16) -> Result<Self> {
        let eeprom = transport.control_in(
            req::EEPROM,
            CALIBRATION_EEPROM_OFFSET,
            0,
            CALIBRATION_EEPROM_LEN,
        )?;
        let mut device = Self {
            transport,
            firmware_version,
            calibration: Calibration::from_eeprom(&eeprom),
            config: AcquisitionConfig {
                sample_rate: SampleRate(1),
                mode: ChannelMode::Dual,
                range: [VoltageRange::V5; 2],
                coupling: [Coupling::Dc; 2],
            },
        };
        // Ogni setter invia il comando e aggiorna la configurazione solo se il comando riesce.
        device.set_sample_rate(SampleRate(1))?;
        device.set_channel_mode(ChannelMode::Dual)?;
        device.set_voltage_range(Channel::Ch1, VoltageRange::V5)?;
        device.set_voltage_range(Channel::Ch2, VoltageRange::V5)?;
        device.set_coupling(Channel::Ch1, Coupling::Dc)?;
        device.set_coupling(Channel::Ch2, Coupling::Dc)?;
        Ok(device)
    }

    pub fn firmware_version(&self) -> u16 {
        self.firmware_version
    }

    /// Versione e capacità dichiarate dal firmware. `None` con un firmware precedente alla 3.00.
    pub fn firmware_info(&self) -> Result<Option<FirmwareInfo>> {
        if self.firmware_version < FIRMWARE_VERSION {
            return Ok(None);
        }
        let data = self.transport.control_in(req::INFO, 0, 0, 4)?;
        match data[..] {
            [lo, hi, capabilities, fault] => Ok(Some(FirmwareInfo {
                version: u16::from_le_bytes([lo, hi]),
                capabilities,
                fault,
            })),
            _ => Err(Error::Protocol(format!("risposta INFO di {} byte", data.len()))),
        }
    }

    /// Volte in cui la FIFO del dispositivo si è riempita dall'ultimo avvio, modulo 65536.
    ///
    /// Ogni evento indica campioni persi: l'host non ha letto abbastanza in fretta.
    /// Il firmware controlla la FIFO a polling, quindi un evento molto breve può sfuggire.
    /// La lettura non ferma lo streaming. `None` con un firmware precedente alla 3.00.
    pub fn overflow_count(&self) -> Result<Option<u16>> {
        if self.firmware_version < FIRMWARE_VERSION {
            return Ok(None);
        }
        let data = self.transport.control_in(req::OVERFLOWS, 0, 0, 2)?;
        match data[..] {
            [lo, hi] => Ok(Some(u16::from_le_bytes([lo, hi]))),
            _ => Err(Error::Protocol(format!("risposta OVERFLOWS di {} byte", data.len()))),
        }
    }

    pub fn calibration(&self) -> &Calibration {
        &self.calibration
    }

    pub fn config(&self) -> AcquisitionConfig {
        self.config
    }

    pub fn set_sample_rate(&mut self, rate: SampleRate) -> Result<()> {
        if SampleRate::from_id(rate.0).is_none() {
            return Err(Error::InvalidArgument(format!(
                "frequenza di campionamento sconosciuta: id {}",
                rate.0
            )));
        }
        self.transport.control_out(req::SAMPLE_RATE, 0, 0, &[rate.0])?;
        self.config.sample_rate = rate;
        Ok(())
    }

    pub fn set_voltage_range(&mut self, channel: Channel, range: VoltageRange) -> Result<()> {
        let request = match channel {
            Channel::Ch1 => req::CH1_RANGE,
            Channel::Ch2 => req::CH2_RANGE,
        };
        self.transport.control_out(request, 0, 0, &[range.code()])?;
        self.config.range[channel_index(channel)] = range;
        Ok(())
    }

    pub fn set_channel_mode(&mut self, mode: ChannelMode) -> Result<()> {
        self.transport.control_out(req::NUM_CHANNELS, 0, 0, &[mode as u8])?;
        self.config.mode = mode;
        Ok(())
    }

    pub fn set_coupling(&mut self, channel: Channel, coupling: Coupling) -> Result<()> {
        // Il firmware riceve un byte solo per entrambi i canali: si invia lo stato completo.
        let mut couplings = self.config.coupling;
        couplings[channel_index(channel)] = coupling;
        self.transport
            .control_out(req::COUPLING, 0, 0, &[coupling_byte(couplings)])?;
        self.config.coupling = couplings;
        Ok(())
    }

    /// Frequenza dell'onda quadra di calibrazione sul connettore della sonda, 32 Hz – 100 kHz.
    pub fn set_calibration_frequency(&mut self, hz: u32) -> Result<()> {
        let code = calibration_code(hz).ok_or_else(|| {
            Error::InvalidArgument(format!("frequenza di calibrazione non supportata: {hz} Hz"))
        })?;
        self.transport.control_out(req::CAL_FREQ, 0, 0, &[code])
    }

    pub fn read_eeprom(&self, offset: u16, len: u16) -> Result<Vec<u8>> {
        self.transport.control_in(req::EEPROM, offset, 0, len)
    }

    /// Apre la coda bulk, poi avvia il campionamento.
    pub fn start_stream(&mut self, config: StreamConfig) -> Result<Stream<T::Bulk>> {
        let mut bulk = self
            .transport
            .open_bulk(BULK_EP_IN, config.transfer_size, config.queue_depth)?;
        // Ogni comando vendor chiude il pacchetto in corso con INPKTEND: il residuo,
        // spesso di lunghezza dispari, sfaserebbe CH1 e CH2.
        bulk.discard_stale(STALE_QUIET)?;
        self.transport.control_out(req::START_STOP, 0, 0, &[1])?;
        let head = self.config.sample_rate.stale_samples_after_restart();
        Ok(Stream::new(bulk, self.config.mode, head, config.timeout))
    }

    pub fn stop(&mut self) -> Result<()> {
        self.transport.control_out(req::START_STOP, 0, 0, &[0])
    }
}

fn channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Ch1 => 0,
        Channel::Ch2 => 1,
    }
}

/// Bit0: CH1 DC, bit4: CH2 DC.
fn coupling_byte(couplings: [Coupling; 2]) -> u8 {
    let mut byte = 0;
    if couplings[0] == Coupling::Dc {
        byte |= 0x01;
    }
    if couplings[1] == Coupling::Dc {
        byte |= 0x10;
    }
    byte
}

/// Codice del firmware per la frequenza di calibrazione, `None` se fuori dagli intervalli.
fn calibration_code(hz: u32) -> Option<u8> {
    let code = if (1000..=100_000).contains(&hz) && hz.is_multiple_of(1000) {
        hz / 1000
    } else if hz == 32 {
        103
    } else if (40..=1000).contains(&hz) && hz.is_multiple_of(10) {
        100 + hz / 10
    } else if (100..=5500).contains(&hz) && hz.is_multiple_of(100) {
        200 + hz / 100
    } else {
        return None;
    };
    u8::try_from(code).ok()
}

/// Richiedono l'Hantek collegato: `cargo test -p h6022-usb --lib -- --ignored hardware`.
#[cfg(test)]
mod hardware {
    use super::*;

    #[test]
    #[ignore]
    fn hardware_recovery_paths_reopen_the_device() {
        drop(Hantek6022::open().expect("apertura"));
        let dev = recover_by_reset().expect("recupero con reset");
        assert_eq!(dev.firmware_version(), FIRMWARE_VERSION);
        drop(dev);
        let dev = reload_firmware(VID_FIRMWARE, RENUMERATION_TIMEOUT).expect("ricarico del firmware");
        assert_eq!(dev.firmware_version(), FIRMWARE_VERSION);
    }
}
