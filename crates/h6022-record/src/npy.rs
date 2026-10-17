//! Scrittore npy.

use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;

use h6022_core::Frame;

use crate::{Format, FrameWriter, RecordSummary, Tally, write_metadata};

/// Lunghezza totale dell'intestazione NPY v1.0 (magia, versione, lunghezza, dizionario).
const HEADER_LEN: usize = 128;

struct NpyWriter {
    out: BufWriter<File>,
    path: PathBuf,
    tally: Tally,
}

pub(crate) fn writer(file: File, path: std::path::PathBuf) -> io::Result<Box<dyn FrameWriter>> {
    let mut out = BufWriter::new(file);
    // Intestazione provvisoria: finish la riscrive con il numero finale di campioni.
    out.write_all(&header(0, 1))?;
    Ok(Box::new(NpyWriter {
        out,
        path,
        tally: Tally::default(),
    }))
}

/// Intestazione NPY v1.0 di 128 byte con dizionario `<f4` in ordine C e forma (righe, colonne).
fn header(rows: u64, cols: u8) -> Vec<u8> {
    let mut dict = format!("{{'descr': '<f4', 'fortran_order': False, 'shape': ({rows}, {cols}), }}");
    // Il campo lunghezza conta dizionario, spazi di riempimento e '\n': 128 - 10 byte.
    let body_len = HEADER_LEN - 10;
    let padding = " ".repeat(body_len - 1 - dict.len());
    dict.push_str(&padding);
    dict.push('\n');
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.extend_from_slice(b"\x93NUMPY\x01\x00");
    out.extend_from_slice(&(body_len as u16).to_le_bytes());
    out.extend_from_slice(dict.as_bytes());
    out
}

impl FrameWriter for NpyWriter {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        self.tally.check(frame)?;
        let ch2 = frame.ch2.as_ref();
        for (i, v1) in frame.ch1.samples.iter().enumerate() {
            self.out.write_all(&v1.to_le_bytes())?;
            if let Some(ch2) = ch2 {
                self.out.write_all(&ch2.samples[i].to_le_bytes())?;
            }
        }
        self.tally.commit(frame);
        Ok(())
    }

    fn finish(self: Box<Self>) -> io::Result<RecordSummary> {
        let this = *self;
        let (_, channels) = this.tally.layout();
        let mut file = this.out.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&header(this.tally.samples_per_channel, channels))?;
        let summary = this.tally.summary(this.path, Format::Npy);
        write_metadata(&summary, this.tally.record_length)?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{frame, scratch};

    #[test]
    fn round_trip_due_frame_due_canali() {
        let dir = scratch("npy");
        let path = dir.join("rec.npy");
        let mut writer = crate::create(&path, Format::Npy).unwrap();
        writer
            .write_frame(&frame(0, 1000.0, 0.0, vec![0.1, 0.2, 0.3], Some(vec![-1.0, -2.0, -3.0])))
            .unwrap();
        writer
            .write_frame(&frame(1, 1000.0, 0.0, vec![0.4, 0.5, 0.6], Some(vec![-4.0, -5.0, -6.0])))
            .unwrap();
        let summary = writer.finish().unwrap();
        assert_eq!(summary.samples_per_channel, 6);

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), HEADER_LEN + 6 * 2 * 4);
        assert_eq!(&bytes[..8], b"\x93NUMPY\x01\x00");
        let dict = std::str::from_utf8(&bytes[10..HEADER_LEN]).unwrap();
        assert!(dict.contains("'shape': (6, 2), }"), "{dict}");
        let data: Vec<f32> = bytes[HEADER_LEN..]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(
            data,
            vec![0.1, -1.0, 0.2, -2.0, 0.3, -3.0, 0.4, -4.0, 0.5, -5.0, 0.6, -6.0]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn intestazione_di_128_byte_allineata() {
        let h = header(2, 2);
        assert_eq!(h.len(), HEADER_LEN);
        assert_eq!(&h[..8], b"\x93NUMPY\x01\x00");
        assert_eq!(u16::from_le_bytes([h[8], h[9]]), 118);
        assert_eq!(h[HEADER_LEN - 1], b'\n');
        let dict = std::str::from_utf8(&h[10..]).unwrap();
        assert_eq!(
            dict.trim_end(),
            "{'descr': '<f4', 'fortran_order': False, 'shape': (2, 2), }"
        );
    }

    #[test]
    fn intestazione_senza_campioni_ha_forma_zero_uno() {
        let h = header(0, 1);
        let dict = std::str::from_utf8(&h[10..]).unwrap();
        assert!(dict.contains("'shape': (0, 1), }"), "{dict}");
    }
}
