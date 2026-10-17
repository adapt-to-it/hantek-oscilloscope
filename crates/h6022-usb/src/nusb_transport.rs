//! Trasporto USB reale su nusb (IOKit su macOS, senza libusb).

use std::time::Duration;

use nusb::MaybeFuture;
use nusb::transfer::{Bulk, ControlIn, ControlOut, ControlType, In, Recipient, TransferError};

use crate::transport::{BulkSource, Transport};
use crate::{Error, Result};

const CONTROL_TIMEOUT: Duration = Duration::from_secs(2);

/// Dispositivo visto sul bus.
#[derive(Debug, Clone)]
pub struct UsbId {
    pub vendor_id: u16,
    pub product_id: u16,
    /// bcdDevice: per il firmware DSO6022BE è la versione del firmware.
    pub device_version: u16,
    pub serial: Option<String>,
}

pub struct NusbTransport {
    device: nusb::Device,
    interface: nusb::Interface,
    id: UsbId,
}

fn usb_err(e: impl std::fmt::Display) -> Error {
    Error::Usb(e.to_string())
}

fn transfer_err(e: TransferError) -> Error {
    match e {
        TransferError::Cancelled => Error::Timeout,
        other => Error::Usb(other.to_string()),
    }
}

/// Elenca i dispositivi con il VID e il PID indicati.
pub fn list(vendor_id: u16, product_id: u16) -> Result<Vec<UsbId>> {
    let devices = nusb::list_devices().wait().map_err(usb_err)?;
    Ok(devices
        .filter(|d| d.vendor_id() == vendor_id && d.product_id() == product_id)
        .map(|d| UsbId {
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            device_version: d.device_version(),
            serial: d.serial_number().map(str::to_owned),
        })
        .collect())
}

/// Reset della porta USB: il dispositivo ripete l'enumerazione, il firmware in RAM resta.
/// macOS rifiuta il reset con un'interfaccia reclamata, quindi apre il dispositivo senza reclamarla.
pub fn reset(vendor_id: u16, product_id: u16) -> Result<()> {
    let info = nusb::list_devices()
        .wait()
        .map_err(usb_err)?
        .find(|d| d.vendor_id() == vendor_id && d.product_id() == product_id)
        .ok_or(Error::NotFound)?;
    info.open().wait().map_err(usb_err)?.reset().wait().map_err(usb_err)
}

impl NusbTransport {
    /// Apre il primo dispositivo con VID e PID indicati e reclama l'interfaccia 0.
    pub fn open(vendor_id: u16, product_id: u16) -> Result<Self> {
        let info = nusb::list_devices()
            .wait()
            .map_err(usb_err)?
            .find(|d| d.vendor_id() == vendor_id && d.product_id() == product_id)
            .ok_or(Error::NotFound)?;
        let id = UsbId {
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            device_version: info.device_version(),
            serial: info.serial_number().map(str::to_owned),
        };
        let device = info.open().wait().map_err(usb_err)?;
        // Su macOS il FX2 senza firmware può risultare non configurato: senza configurazione
        // l'interfaccia 0 non esiste.
        if device.active_configuration().map(|c| c.configuration_value()) != Ok(1) {
            device.set_configuration(1).wait().map_err(usb_err)?;
        }
        let interface = device.claim_interface(0).wait().map_err(usb_err)?;
        Ok(Self { device, interface, id })
    }

    pub fn id(&self) -> &UsbId {
        &self.id
    }
}

impl Transport for NusbTransport {
    type Bulk = NusbBulk;

    fn control_out(&self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()> {
        let setup = ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request,
            value,
            index,
            data,
        };
        self.device.control_out(setup, CONTROL_TIMEOUT).wait().map_err(transfer_err)
    }

    fn control_in(&self, request: u8, value: u16, index: u16, len: u16) -> Result<Vec<u8>> {
        let setup = ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request,
            value,
            index,
            length: len,
        };
        self.device.control_in(setup, CONTROL_TIMEOUT).wait().map_err(transfer_err)
    }

    fn open_bulk(&self, endpoint: u8, transfer_size: usize, queue_depth: usize) -> Result<NusbBulk> {
        let mut ep = self.interface.endpoint::<Bulk, In>(endpoint).map_err(usb_err)?;
        // nusb richiede una lunghezza multipla della dimensione massima del pacchetto.
        let packet = ep.max_packet_size().max(1);
        let size = transfer_size.div_ceil(packet) * packet;
        for _ in 0..queue_depth.max(1) {
            let buf = ep.allocate(size);
            ep.submit(buf);
        }
        Ok(NusbBulk { ep })
    }
}

pub struct NusbBulk {
    ep: nusb::Endpoint<Bulk, In>,
}

impl BulkSource for NusbBulk {
    fn read_into(&mut self, out: &mut Vec<u8>, timeout: Duration) -> Result<usize> {
        let completion = self.ep.wait_next_complete(timeout).ok_or(Error::Timeout)?;
        let n = completion.actual_len;
        out.extend_from_slice(&completion.buffer[..n]);
        let status = completion.status;
        let mut buf = completion.buffer;
        let size = buf.requested_len();
        buf.clear();
        buf.set_requested_len(size);
        self.ep.submit(buf);
        status.map_err(transfer_err)?;
        Ok(n)
    }

    fn discard_stale(&mut self, quiet: Duration) -> Result<usize> {
        self.discard(quiet)
    }
}

impl NusbBulk {
    fn discard(&mut self, quiet: Duration) -> Result<usize> {
        let mut sink = Vec::new();
        let mut total = 0;
        loop {
            sink.clear();
            match self.read_into(&mut sink, quiet) {
                Ok(n) => total += n,
                Err(Error::Timeout) => return Ok(total),
                Err(e) => return Err(e),
            }
        }
    }
}

impl Drop for NusbBulk {
    fn drop(&mut self) {
        self.ep.cancel_all();
    }
}
