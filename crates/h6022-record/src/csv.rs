//! Scrittore csv.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use h6022_core::Frame;

use crate::{Format, FrameWriter, RecordSummary, Tally, write_metadata};

struct CsvWriter {
    out: BufWriter<File>,
    path: PathBuf,
    tally: Tally,
    header: bool,
}

pub(crate) fn writer(file: File, path: std::path::PathBuf) -> io::Result<Box<dyn FrameWriter>> {
    Ok(Box::new(CsvWriter {
        out: BufWriter::new(file),
        path,
        tally: Tally::default(),
        header: false,
    }))
}

/// Intestazione con una colonna per canale; il numero di canali arriva con il primo frame.
fn write_header(out: &mut impl Write, channels: u8) -> io::Result<()> {
    if channels == 2 {
        writeln!(out, "frame,t_s,ch1_v,ch2_v")
    } else {
        writeln!(out, "frame,t_s,ch1_v")
    }
}

impl FrameWriter for CsvWriter {
    fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        let (rate, channels) = self.tally.check(frame)?;
        if !self.header {
            write_header(&mut self.out, channels)?;
            self.header = true;
        }
        let index = self.tally.frames;
        for (i, v1) in frame.ch1.samples.iter().enumerate() {
            let t = frame.ch1.t0 + i as f64 / rate;
            write!(self.out, "{index},{t:.9},{v1:.6}")?;
            if let Some(ch2) = &frame.ch2 {
                write!(self.out, ",{:.6}", ch2.samples[i])?;
            }
            writeln!(self.out)?;
        }
        self.tally.commit(frame);
        Ok(())
    }

    fn finish(self: Box<Self>) -> io::Result<RecordSummary> {
        let mut this = *self;
        if !this.header {
            write_header(&mut this.out, 1)?;
        }
        this.out.flush()?;
        let summary = this.tally.summary(this.path, Format::Csv);
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
        let dir = scratch("csv");
        let path = dir.join("rec.csv");
        let mut writer = crate::create(&path, Format::Csv).unwrap();
        writer
            .write_frame(&frame(0, 1000.0, -0.001, vec![0.1, 0.2, 0.3], Some(vec![-1.0, -2.0, -3.0])))
            .unwrap();
        writer
            .write_frame(&frame(1, 1000.0, 0.0, vec![0.4, 0.5, 0.6], Some(vec![-4.0, -5.0, -6.0])))
            .unwrap();
        let summary = writer.finish().unwrap();
        assert_eq!(summary.frames, 2);
        assert_eq!(summary.channels, 2);
        assert_eq!(summary.samples_per_channel, 6);

        let text = std::fs::read_to_string(&path).unwrap();
        let expected = [
            "frame,t_s,ch1_v,ch2_v",
            "0,-0.001000000,0.100000,-1.000000",
            "0,0.000000000,0.200000,-2.000000",
            "0,0.001000000,0.300000,-3.000000",
            "1,0.000000000,0.400000,-4.000000",
            "1,0.001000000,0.500000,-5.000000",
            "1,0.002000000,0.600000,-6.000000",
        ]
        .join("\n")
            + "\n";
        assert_eq!(text, expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn intestazione_con_uno_o_due_canali() {
        let mut buf = Vec::new();
        write_header(&mut buf, 1).unwrap();
        assert_eq!(buf, b"frame,t_s,ch1_v\n");
        buf.clear();
        write_header(&mut buf, 2).unwrap();
        assert_eq!(buf, b"frame,t_s,ch1_v,ch2_v\n");
    }
}
