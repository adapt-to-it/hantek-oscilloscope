//! Scrittore wav.

use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;

use h6022_core::Frame;

use crate::{Format, FrameWriter, RecordSummary, Tally, invalid_input, write_metadata};

/// Dati massimi: la dimensione RIFF (dati + 36) deve stare in u32.
const MAX_DATA: u64 = u32::MAX as u64 - 36;

struct WavWriter {
    out: BufWriter<File>,
    path: PathBuf,
    tally: Tally,
}

pub(crate) fn writer(file: File, path: std::path::PathBuf) -> io::Result<Box<dyn FrameWriter>> {
    let mut out = BufWriter::new(file);
    // Intestazione provvisoria: finish la riscrive con frequenza, canali e dati finali.
    out.write_all(&header(1, 0, 0))?;
    Ok(Box::new(WavWriter {
        out,
        path,
        tally: Tally::default(),
    }))
}

/// Intestazione RIFF/WAVE con chunk fmt da 16 byte, float IEEE (formato 3) a 32 bit.
fn header(channels: u16, rate: u32, data_bytes: u32) -> Vec<u8> {
    let block_align = channels * 4;
    let byte_rate = rate.saturating_mul(u32::from(block_align));
    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    h.extend_from_slice(b"WAVEfmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&3u16.to_le_bytes());
    h.extend_from_slice(&channels.to_le_bytes());
    h.extend_from_slice(&rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_align.to_le_bytes());
    h.extend_from_slice(&32u16.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_bytes.to_le_bytes());
    h
}

impl FrameWriter for WavWriter {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        let (_, channels) = self.tally.check(frame)?;
        let samples = frame.ch1.samples.len() as u64;
        let total = (self.tally.samples_per_channel + samples) * u64::from(channels) * 4;
        if total > MAX_DATA {
            return Err(invalid_input("WAV oltre 4 GiB"));
        }
        let ch2 = frame.ch2.as_ref();
        // WAV intercalato: per ogni campione, un valore per canale.
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
        let (rate, channels) = this.tally.layout();
        let data = this.tally.samples_per_channel * u64::from(channels) * 4;
        let data = u32::try_from(data).map_err(|_| invalid_input("WAV oltre 4 GiB"))?;
        let mut file = this.out.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&header(u16::from(channels), rate.round() as u32, data))?;
        let summary = this.tally.summary(this.path, Format::Wav);
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
        let dir = scratch("wav");
        let path = dir.join("rec.wav");
        let mut writer = crate::create(&path, Format::Wav).unwrap();
        writer
            .write_frame(&frame(0, 1000.0, 0.0, vec![0.1, 0.2, 0.3], Some(vec![-1.0, -2.0, -3.0])))
            .unwrap();
        writer
            .write_frame(&frame(1, 1000.0, 0.0, vec![0.4, 0.5, 0.6], Some(vec![-4.0, -5.0, -6.0])))
            .unwrap();
        let summary = writer.finish().unwrap();
        assert_eq!(summary.samples_per_channel, 6);

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 44 + 6 * 2 * 4);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 36 + 48);
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 3);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 1000);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 48);
        let data: Vec<f32> = bytes[44..].as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
        assert_eq!(
            data,
            vec![0.1, -1.0, 0.2, -2.0, 0.3, -3.0, 0.4, -4.0, 0.5, -5.0, 0.6, -6.0]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn intestazione_stereo_float32() {
        let h = header(2, 1_000_000, 16);
        assert_eq!(h.len(), 44);
        assert_eq!(&h[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 52);
        assert_eq!(&h[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(h[16..20].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(h[20..22].try_into().unwrap()), 3);
        assert_eq!(u16::from_le_bytes(h[22..24].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(h[24..28].try_into().unwrap()), 1_000_000);
        assert_eq!(u32::from_le_bytes(h[28..32].try_into().unwrap()), 8_000_000);
        assert_eq!(u16::from_le_bytes(h[32..34].try_into().unwrap()), 8);
        assert_eq!(u16::from_le_bytes(h[34..36].try_into().unwrap()), 32);
        assert_eq!(&h[36..40], b"data");
        assert_eq!(u32::from_le_bytes(h[40..44].try_into().unwrap()), 16);
    }
}
