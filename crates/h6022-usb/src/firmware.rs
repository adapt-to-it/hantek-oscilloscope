//! Caricamento del firmware nella RAM del FX2LP.

use crate::ihex::{self, HexRecord};
use crate::Result;
use crate::transport::Transport;
use crate::types::{req, CPUCS_ADDR};

/// Firmware DSO6022BE compilato da `firmware/dso6022be`.
pub const DSO6022BE_HEX: &str = include_str!("../../../firmware/dso6022be/dso6022be-firmware.hex");

/// Massimo di byte per trasferimento di scrittura RAM.
const MAX_TRANSFER: usize = 1024;

/// Ferma l'8051, scrive i record nella RAM, riavvia l'8051.
/// Dopo il riavvio il dispositivo si disconnette e si rienumera con `VID_FIRMWARE`.
pub fn upload<T: Transport>(transport: &T, hex: &str) -> Result<()> {
    // Si legge tutto il file prima di fermare l'8051: un errore di formato non lascia il dispositivo fermo.
    let records = ihex::parse(hex)?;

    transport.control_out(req::FIRMWARE, CPUCS_ADDR, 0, &[1])?;
    for chunk in merge_records(&records) {
        transport.control_out(req::FIRMWARE, chunk.address, 0, &chunk.data)?;
    }
    transport.control_out(req::FIRMWARE, CPUCS_ADDR, 0, &[0])?;
    Ok(())
}

/// Unisce i record contigui in blocchi di al più `MAX_TRANSFER` byte. Salta i record vuoti.
fn merge_records(records: &[HexRecord]) -> Vec<HexRecord> {
    let mut merged: Vec<HexRecord> = Vec::new();
    for record in records {
        if record.data.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            let end = u32::from(last.address) + last.data.len() as u32;
            if u32::from(record.address) == end && last.data.len() + record.data.len() <= MAX_TRANSFER {
                last.data.extend_from_slice(&record.data);
                continue;
            }
        }
        merged.push(record.clone());
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use crate::transport::BulkSource;
    use std::cell::RefCell;
    use std::time::Duration;

    /// Chiamata registrata: (request, value, index, data).
    type Call = (u8, u16, u16, Vec<u8>);

    struct NoBulk;

    impl BulkSource for NoBulk {
        fn read_into(&mut self, _out: &mut Vec<u8>, _timeout: Duration) -> Result<usize> {
            Err(Error::Timeout)
        }
    }

    /// Trasporto simulato che registra i trasferimenti di controllo OUT.
    #[derive(Default)]
    struct Recorder {
        calls: RefCell<Vec<Call>>,
    }

    impl Transport for Recorder {
        type Bulk = NoBulk;

        fn control_out(&self, request: u8, value: u16, index: u16, data: &[u8]) -> Result<()> {
            self.calls.borrow_mut().push((request, value, index, data.to_vec()));
            Ok(())
        }

        fn control_in(&self, _request: u8, _value: u16, _index: u16, _len: u16) -> Result<Vec<u8>> {
            Err(Error::Protocol("control_in non previsto nel test".to_string()))
        }

        fn open_bulk(&self, _endpoint: u8, _transfer_size: usize, _queue_depth: usize) -> Result<NoBulk> {
            Err(Error::Protocol("open_bulk non previsto nel test".to_string()))
        }
    }

    /// Riga Intel HEX di tipo dati, con checksum calcolato.
    fn hex_line(address: u16, data: &[u8]) -> String {
        let mut bytes = vec![data.len() as u8, (address >> 8) as u8, address as u8, 0x00];
        bytes.extend_from_slice(data);
        let sum = bytes.iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
        bytes.push(sum.wrapping_neg());
        let mut line = String::from(":");
        for b in &bytes {
            line.push_str(&format!("{b:02X}"));
        }
        line
    }

    #[test]
    fn real_firmware_parses() {
        assert!(ihex::parse(DSO6022BE_HEX).is_ok());
    }

    #[test]
    fn real_firmware_halts_then_runs_cpu() {
        let rec = Recorder::default();
        upload(&rec, DSO6022BE_HEX).unwrap();
        let calls = rec.calls.borrow();
        assert_eq!(calls.first(), Some(&(req::FIRMWARE, CPUCS_ADDR, 0, vec![1])));
        assert_eq!(calls.last(), Some(&(req::FIRMWARE, CPUCS_ADDR, 0, vec![0])));
    }

    #[test]
    fn contiguous_records_are_merged() {
        let hex = format!(
            "{}\n{}\n{}\n:00000001FF\n",
            hex_line(0x0100, &[1, 2, 3]),
            hex_line(0x0103, &[4]),
            hex_line(0x0200, &[9]),
        );
        let rec = Recorder::default();
        upload(&rec, &hex).unwrap();
        assert_eq!(
            *rec.calls.borrow(),
            vec![
                (req::FIRMWARE, CPUCS_ADDR, 0, vec![1]),
                (req::FIRMWARE, 0x0100, 0, vec![1, 2, 3, 4]),
                (req::FIRMWARE, 0x0200, 0, vec![9]),
                (req::FIRMWARE, CPUCS_ADDR, 0, vec![0]),
            ]
        );
    }

    #[test]
    fn long_runs_are_split_at_1024_bytes() {
        let block = [0xAA; 255];
        let hex = format!(
            "{}\n{}\n{}\n{}\n{}\n:00000001FF\n",
            hex_line(0x0000, &block),
            hex_line(0x00FF, &block),
            hex_line(0x01FE, &block),
            hex_line(0x02FD, &block),
            hex_line(0x03FC, &block),
        );
        let rec = Recorder::default();
        upload(&rec, &hex).unwrap();
        let calls = rec.calls.borrow();
        assert_eq!(calls.len(), 4);
        assert_eq!((calls[1].1, calls[1].3.len()), (0x0000, 1020));
        assert_eq!((calls[2].1, calls[2].3.len()), (0x03FC, 255));
    }

    #[test]
    fn invalid_hex_does_not_touch_device() {
        let rec = Recorder::default();
        assert!(upload(&rec, "garbage").is_err());
        assert!(rec.calls.borrow().is_empty());
    }
}
