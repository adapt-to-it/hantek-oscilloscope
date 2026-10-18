//! Esportazione di tracce logiche e annotazioni: VCD e sessione sigrok `.sr`.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::OnceLock;

use crate::{Annotation, LogicTrace};

/// Value Change Dump con timescale 1 ns. Un segnale `wire 1` per traccia, con il nome dato.
/// Le annotazioni diventano un segnale `string` "decoder" con il testo all'inizio di ognuna.
/// Le tracce devono avere la stessa frequenza e lunghezza.
pub fn write_vcd<W: Write>(mut out: W, traces: &[(&str, &LogicTrace)], annotations: &[Annotation]) -> io::Result<()> {
    check_traces(traces)?;
    if traces.len() + 1 > 94 {
        return Err(invalid("troppi segnali per gli identificativi VCD"));
    }
    let sample_rate = traces.first().map_or(1.0, |(_, t)| t.sample_rate);
    let t0 = traces.first().map_or(0.0, |(_, t)| t.t0);
    let ann_id = vcd_id(traces.len());

    let mut events: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    let mut initial = Vec::with_capacity(traces.len());
    for (k, (_, t)) in traces.iter().enumerate() {
        let id = vcd_id(k);
        // Il livello al campione 0 tiene conto delle transizioni nello stesso campione.
        let mut level = t.level_at(0);
        initial.push(format!("{}{}", bit(level), id));
        for &idx in t.transitions.iter().filter(|&&i| i > 0) {
            level = !level;
            events
                .entry(to_ns(idx as f64 / sample_rate))
                .or_default()
                .push(format!("{}{}", bit(level), id));
        }
    }
    for a in annotations {
        events
            .entry(to_ns(a.start_s - t0))
            .or_default()
            .push(format!("s{} {}", token(&a.text), ann_id));
    }

    let mut buf: Vec<u8> = Vec::new();
    writeln!(buf, "$date\n  h6022\n$end")?;
    writeln!(buf, "$version\n  h6022-decode\n$end")?;
    writeln!(buf, "$timescale 1 ns $end")?;
    writeln!(buf, "$scope module h6022 $end")?;
    for (k, (name, _)) in traces.iter().enumerate() {
        writeln!(buf, "$var wire 1 {} {} $end", vcd_id(k), token(name))?;
    }
    writeln!(buf, "$var string 1 {} decoder $end", ann_id)?;
    writeln!(buf, "$upscope $end")?;
    writeln!(buf, "$enddefinitions $end")?;
    writeln!(buf, "#0")?;
    writeln!(buf, "$dumpvars")?;
    for line in &initial {
        writeln!(buf, "{}", line)?;
    }
    writeln!(buf, "$end")?;
    for (t, lines) in &events {
        if *t != 0 {
            writeln!(buf, "#{}", t)?;
        }
        for line in lines {
            writeln!(buf, "{}", line)?;
        }
    }
    out.write_all(&buf)
}

/// Sessione sigrok: archivio zip non compresso (metodo store, CRC-32) con i file
/// `version` ("2"), `metadata` (formato ini: [device 1], samplerate, total probes, probeN, unitsize=1)
/// e `logic-1-1` (un byte per campione, bit k = traccia k). Si apre con PulseView.
pub fn write_sigrok<W: Write>(mut out: W, traces: &[(&str, &LogicTrace)]) -> io::Result<()> {
    if traces.is_empty() {
        return Err(invalid("nessuna traccia da esportare"));
    }
    if traces.len() > 8 {
        return Err(invalid("sigrok supporta al massimo 8 tracce"));
    }
    check_traces(traces)?;
    let first = traces[0].1;

    let mut logic = vec![0u8; first.len];
    for (k, (_, t)) in traces.iter().enumerate() {
        for (start, end, level) in t.runs() {
            let end = end.min(first.len);
            if level && start < end {
                for byte in &mut logic[start..end] {
                    *byte |= 1u8 << k;
                }
            }
        }
    }

    let mut meta = String::from("[global]\nsigrok version=0.5.0\n\n[device 1]\ncapturefile=logic-1\n");
    meta.push_str(&format!("total probes={}\n", traces.len()));
    meta.push_str(&format!("samplerate={} Hz\n", first.sample_rate));
    for (k, (name, _)) in traces.iter().enumerate() {
        meta.push_str(&format!("probe{}={}\n", k + 1, ini_value(name)));
    }
    meta.push_str("unitsize=1\n");

    let files: [(&str, Vec<u8>); 3] = [
        ("version", b"2".to_vec()),
        ("metadata", meta.into_bytes()),
        ("logic-1-1", logic),
    ];

    let mut archive: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    for (name, data) in &files {
        let offset = to_u32(archive.len())?;
        let crc = crc32(data);
        let size = to_u32(data.len())?;
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;

        // Local file header.
        put32(&mut archive, 0x0403_4b50);
        put16(&mut archive, 20); // versione
        put16(&mut archive, 0); // flag
        put16(&mut archive, 0); // metodo: store
        put16(&mut archive, 0); // ora
        put16(&mut archive, 0); // data
        put32(&mut archive, crc);
        put32(&mut archive, size);
        put32(&mut archive, size);
        put16(&mut archive, name_len);
        put16(&mut archive, 0); // extra
        archive.extend_from_slice(name.as_bytes());
        archive.extend_from_slice(data);

        // Voce della central directory.
        put32(&mut central, 0x0201_4b50);
        put16(&mut central, 20); // versione di creazione
        put16(&mut central, 20); // versione richiesta
        put16(&mut central, 0); // flag
        put16(&mut central, 0); // metodo
        put16(&mut central, 0); // ora
        put16(&mut central, 0); // data
        put32(&mut central, crc);
        put32(&mut central, size);
        put32(&mut central, size);
        put16(&mut central, name_len);
        put16(&mut central, 0); // extra
        put16(&mut central, 0); // commento
        put16(&mut central, 0); // disco
        put16(&mut central, 0); // attributi interni
        put32(&mut central, 0); // attributi esterni
        put32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }

    let cd_offset = to_u32(archive.len())?;
    let cd_size = to_u32(central.len())?;
    let count = u16::try_from(files.len()).map_err(|_| too_big())?;
    archive.extend_from_slice(&central);

    // End of central directory.
    put32(&mut archive, 0x0605_4b50);
    put16(&mut archive, 0); // disco
    put16(&mut archive, 0); // disco della central directory
    put16(&mut archive, count);
    put16(&mut archive, count);
    put32(&mut archive, cd_size);
    put32(&mut archive, cd_offset);
    put16(&mut archive, 0); // commento

    out.write_all(&archive)
}

/// CRC-32 IEEE (polinomio riflesso 0xEDB88320), usato dallo zip.
pub fn crc32(data: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *slot = c;
        }
        t
    });
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

fn check_traces(traces: &[(&str, &LogicTrace)]) -> io::Result<()> {
    let Some((_, first)) = traces.first() else {
        return Ok(());
    };
    if !(first.sample_rate.is_finite() && first.sample_rate > 0.0) {
        return Err(invalid("frequenza di campionamento non valida"));
    }
    if traces
        .iter()
        .any(|(_, t)| t.sample_rate != first.sample_rate || t.len != first.len)
    {
        return Err(invalid("tracce con frequenza o lunghezza diverse"));
    }
    Ok(())
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

fn too_big() -> io::Error {
    invalid("dati troppo grandi per lo zip")
}

fn to_u32(n: usize) -> io::Result<u32> {
    u32::try_from(n).map_err(|_| too_big())
}

fn put16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// Identificativo VCD della variabile `k`: '!' più k.
fn vcd_id(k: usize) -> char {
    (b'!' + k as u8) as char
}

fn bit(level: bool) -> char {
    if level { '1' } else { '0' }
}

/// Secondi in nanosecondi arrotondati, mai negativi.
fn to_ns(seconds: f64) -> u64 {
    (seconds * 1e9).round().max(0.0) as u64
}

/// Testo senza spazi per i valori VCD e i nomi di variabile.
fn token(s: &str) -> String {
    s.chars().map(|c| if c.is_whitespace() { '_' } else { c }).collect()
}

/// Valore ini su una sola riga.
fn ini_value(s: &str) -> String {
    s.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    fn sample_traces() -> (LogicTrace, LogicTrace) {
        (
            LogicTrace::from_transitions(1e6, 8, false, vec![2, 5]),
            LogicTrace::from_transitions(1e6, 8, true, vec![3]),
        )
    }

    fn u16_at(b: &[u8], p: usize) -> u16 {
        u16::from_le_bytes([b[p], b[p + 1]])
    }

    fn u32_at(b: &[u8], p: usize) -> u32 {
        u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
    }

    #[test]
    fn crc32_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn vcd_header_changes_and_annotation() {
        let (a, b) = sample_traces();
        let ann = [Annotation::new(4e-6, 6e-6, "data", "0x41 'A'")];
        let mut out = Vec::new();
        write_vcd(&mut out, &[("a", &a), ("b", &b)], &ann).unwrap();
        let text = String::from_utf8(out).unwrap();
        let expected = r##"$date
  h6022
$end
$version
  h6022-decode
$end
$timescale 1 ns $end
$scope module h6022 $end
$var wire 1 ! a $end
$var wire 1 " b $end
$var string 1 # decoder $end
$upscope $end
$enddefinitions $end
#0
$dumpvars
0!
1"
$end
#2000
1!
#3000
0"
#4000
s0x41_'A' #
#5000
0!
"##;
        assert_eq!(text, expected);
    }

    #[test]
    fn sigrok_archive_layout_and_contents() {
        let (a, b) = sample_traces();
        let mut out = Vec::new();
        write_sigrok(&mut out, &[("a", &a), ("b", &b)]).unwrap();
        let ar = out;

        // End of central directory in fondo, con 3 voci.
        let eocd = ar.len() - 22;
        assert_eq!(u32_at(&ar, eocd), 0x0605_4b50);
        assert_eq!(u16_at(&ar, eocd + 10), 3);
        let cd_off = u32_at(&ar, eocd + 16) as usize;

        let mut pos = cd_off;
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        for _ in 0..3 {
            assert_eq!(u32_at(&ar, pos), 0x0201_4b50);
            let crc = u32_at(&ar, pos + 16);
            let csize = u32_at(&ar, pos + 20) as usize;
            let nlen = u16_at(&ar, pos + 28) as usize;
            let loff = u32_at(&ar, pos + 42) as usize;
            let name = String::from_utf8(ar[pos + 46..pos + 46 + nlen].to_vec()).unwrap();

            // Local header alla posizione indicata dalla central directory.
            assert_eq!(u32_at(&ar, loff), 0x0403_4b50);
            assert_eq!(u32_at(&ar, loff + 14), crc);
            let lnlen = u16_at(&ar, loff + 26) as usize;
            assert_eq!(&ar[loff + 30..loff + 30 + lnlen], name.as_bytes());
            let ds = loff + 30 + lnlen;
            let data = ar[ds..ds + csize].to_vec();
            assert_eq!(crc32(&data), crc);
            entries.push((name, data));
            pos += 46 + nlen;
        }

        let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["version", "metadata", "logic-1-1"]);
        assert_eq!(entries[0].1, b"2");

        let meta = String::from_utf8(entries[1].1.clone()).unwrap();
        assert!(meta.contains("total probes=2\n"));
        assert!(meta.contains("samplerate=1000000 Hz\n"));
        assert!(meta.contains("probe1=a\n"));
        assert!(meta.contains("probe2=b\n"));
        assert!(meta.contains("unitsize=1\n"));

        // a: 0 0 1 1 1 0 0 0 (bit 0); b: 1 1 1 0 0 0 0 0 (bit 1).
        assert_eq!(entries[2].1, vec![2, 2, 3, 1, 1, 0, 0, 0]);
    }

    #[test]
    fn mismatched_traces_are_invalid_input() {
        let a = LogicTrace::from_transitions(1e6, 8, false, vec![]);
        let short = LogicTrace::from_transitions(1e6, 4, false, vec![]);
        let err = write_vcd(Vec::new(), &[("a", &a), ("s", &short)], &[]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        let err = write_sigrok(Vec::new(), &[("a", &a), ("s", &short)]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn sigrok_rejects_more_than_eight_traces() {
        let a = LogicTrace::from_transitions(1e6, 8, false, vec![]);
        let nine: Vec<(&str, &LogicTrace)> = (0..9).map(|_| ("x", &a)).collect();
        let err = write_sigrok(Vec::new(), &nine).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}
