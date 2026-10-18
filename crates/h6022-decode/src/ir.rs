//! Telecomandi a infrarossi, segnale già demodulato dal ricevitore (attivo basso).

use crate::{Annotation, LogicTrace};

/// Segmento `(inizio, fine_esclusa, livello)`, come `LogicTrace::runs`.
type Run = (usize, usize, bool);

/// Mezzo bit RC5 in microsecondi.
const RC5_HALF_US: f64 = 889.0;

/// Durata di `[start, end)` campioni in microsecondi.
fn dur_us(trace: &LogicTrace, start: usize, end: usize) -> f64 {
    (end - start) as f64 / trace.sample_rate * 1e6
}

/// Vero se `d` è entro ±25 % di `nominal` (entrambi in microsecondi).
fn near(d: f64, nominal: f64) -> bool {
    (d - nominal).abs() <= nominal * 0.25
}

/// NEC: impulso 9 ms + pausa 4,5 ms, poi 32 bit (impulso 562 µs; pausa 562 µs = 0, 1687 µs = 1),
/// LSB per primo: indirizzo, indirizzo negato, comando, comando negato. Ripetizione: 9 ms + 2,25 ms.
/// Annotazioni "start", "address", "command" (error se il negato non torna), "command" "repeat".
/// Tolleranza ±25 %.
pub fn decode_nec(trace: &LogicTrace) -> Vec<Annotation> {
    let runs = trace.runs();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < runs.len() {
        let (s0, e0, l0) = runs[i];
        let (s1, e1, l1) = runs[i + 1];
        // Impulso basso di 9 ms seguito da una pausa alta.
        if l0 || !l1 || !near(dur_us(trace, s0, e0), 9000.0) {
            i += 1;
            continue;
        }
        let pausa = dur_us(trace, s1, e1);
        if near(pausa, 4500.0) {
            match read_nec_bits(trace, &runs, i + 2) {
                Ok(bytes) => {
                    push_nec_frame(trace, &runs, i, bytes, &mut out);
                    i += 2 + 64;
                }
                Err(j) => {
                    out.push(
                        Annotation::new(trace.time(s0), trace.time(e1), "error", "NEC: temporizzazione non valida")
                            .as_error(),
                    );
                    i = j;
                }
            }
        } else if near(pausa, 2250.0) {
            out.push(Annotation::new(trace.time(s0), trace.time(e1), "start", "NEC ripetizione"));
            out.push(Annotation::new(trace.time(s0), trace.time(e1), "command", "repeat"));
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Legge 32 coppie impulso/pausa a partire da `from` (LSB per primo) e restituisce i 4 byte,
/// oppure l'indice della prima run che non rispetta la temporizzazione.
fn read_nec_bits(trace: &LogicTrace, runs: &[Run], from: usize) -> Result<[u8; 4], usize> {
    let mut bytes = [0u8; 4];
    for n in 0..32 {
        let j = from + 2 * n;
        let &(bs, be, bl) = runs.get(j).ok_or(j)?;
        let &(ss, se, sl) = runs.get(j + 1).ok_or(j)?;
        if bl || !sl || !near(dur_us(trace, bs, be), 562.0) {
            return Err(j);
        }
        let pausa = dur_us(trace, ss, se);
        let bit: u8 = if near(pausa, 562.0) {
            0
        } else if near(pausa, 1687.0) {
            1
        } else {
            return Err(j);
        };
        bytes[n / 8] |= bit << (n % 8);
    }
    Ok(bytes)
}

/// Tempi `(inizio, fine)` in secondi dei bit `a..=b` di un frame che inizia a `from`.
fn bits_span(trace: &LogicTrace, runs: &[Run], from: usize, a: usize, b: usize) -> (f64, f64) {
    (trace.time(runs[from + 2 * a].0), trace.time(runs[from + 2 * b + 1].1))
}

fn push_nec_frame(trace: &LogicTrace, runs: &[Run], lead: usize, bytes: [u8; 4], out: &mut Vec<Annotation>) {
    let from = lead + 2;
    let [addr, addr_neg, cmd, cmd_neg] = bytes;
    out.push(Annotation::new(
        trace.time(runs[lead].0),
        trace.time(runs[lead + 1].1),
        "start",
        "NEC",
    ));
    let (s, e) = bits_span(trace, runs, from, 0, 15);
    let mut a = Annotation::new(s, e, "address", format!("0x{addr:02X}")).with_value(u64::from(addr));
    if addr_neg != !addr {
        a = a.as_error();
    }
    out.push(a);
    let (s, e) = bits_span(trace, runs, from, 16, 31);
    let mut c = Annotation::new(s, e, "command", format!("0x{cmd:02X}")).with_value(u64::from(cmd));
    if cmd_neg != !cmd {
        c = c.as_error();
    }
    out.push(c);
}

/// RC5: Manchester con mezzo bit 889 µs, 14 bit: 2 start, toggle, 5 indirizzo, 6 comando, MSB per primo.
/// Annotazioni "start", "bit" per il toggle, "address", "command". Tolleranza ±25 %.
pub fn decode_rc5(trace: &LogicTrace) -> Vec<Annotation> {
    let half = RC5_HALF_US * 1e-6 * trace.sample_rate;
    let tol = half * 0.25;
    let mut out = Vec::new();
    let mut k = 0;
    while k < trace.transitions.len() {
        let t0 = trace.transitions[k];
        // Il primo fronte di un frame cade a metà del bit di start (logico 1, basso→alto):
        // il ricevitore scende, quindi dopo il fronte il livello è basso.
        let frame = if trace.level_at(t0) { None } else { read_rc5_bits(trace, t0, half, tol) };
        match frame {
            Some((bits, end)) => {
                push_rc5_frame(trace, t0, half, &bits, &mut out);
                k = trace.transitions.partition_point(|&t| (t as f64) <= end);
            }
            None => k += 1,
        }
    }
    out
}

/// Legge 14 bit RC5 a partire dal fronte `t0` (metà del primo bit). Ogni bit ha un fronte a metà
/// bit, dentro ±`tol`; il bit è il livello logico (negato del ricevitore) dopo quel fronte.
/// Restituisce i bit e il limite (in campioni) oltre cui i fronti non appartengono al frame.
fn read_rc5_bits(trace: &LogicTrace, t0: usize, half: f64, tol: f64) -> Option<([u8; 14], f64)> {
    let mut bits = [0u8; 14];
    let mut last_mid = 0.0;
    for (n, slot) in bits.iter_mut().enumerate() {
        let mid = t0 as f64 + 2.0 * n as f64 * half;
        let from = (mid - tol).ceil().max(0.0) as usize;
        let t = trace.next_transition(from)?;
        if (t as f64) > mid + tol {
            return None;
        }
        *slot = u8::from(!trace.level_at(t));
        last_mid = mid;
    }
    Some((bits, last_mid + tol))
}

fn push_rc5_frame(trace: &LogicTrace, t0: usize, half: f64, bits: &[u8; 14], out: &mut Vec<Annotation>) {
    let base = t0 as f64;
    // Bit n centrato su base + 2n·half: lo span di a..=b va da metà bit a a metà bit b.
    let at = |x: f64| trace.t0 + x / trace.sample_rate;
    let span = |a: usize, b: usize| (at(base + 2.0 * a as f64 * half - half), at(base + 2.0 * b as f64 * half + half));
    let value = |a: usize, b: usize| bits[a..=b].iter().fold(0u64, |v, &x| (v << 1) | u64::from(x));

    let (s, e) = span(0, 1);
    out.push(Annotation::new(s, e, "start", "RC5 start"));
    let (s, e) = span(2, 2);
    let toggle = value(2, 2);
    out.push(Annotation::new(s, e, "bit", format!("toggle {toggle}")).with_value(toggle));
    let (s, e) = span(3, 7);
    let addr = value(3, 7);
    out.push(Annotation::new(s, e, "address", format!("0x{addr:02X}")).with_value(addr));
    let (s, e) = span(8, 13);
    let cmd = value(8, 13);
    out.push(Annotation::new(s, e, "command", format!("0x{cmd:02X}")).with_value(cmd));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traccia a 1 MHz da segmenti `(livello, campioni)`.
    fn trace(segs: &[(bool, usize)]) -> LogicTrace {
        let initial = segs[0].0;
        let mut transitions = Vec::new();
        let mut level = initial;
        let mut pos = 0;
        for &(lvl, len) in segs {
            if lvl != level {
                transitions.push(pos);
                level = lvl;
            }
            pos += len;
        }
        LogicTrace::from_transitions(1_000_000.0, pos, initial, transitions)
    }

    /// Frame NEC: 4 byte LSB per primo (indirizzo, indirizzo negato, comando, comando negato).
    fn nec_frame(bytes: [u8; 4]) -> Vec<(bool, usize)> {
        let word = u32::from_le_bytes(bytes);
        let mut segs = vec![(false, 9000), (true, 4500)];
        for i in 0..32 {
            segs.push((false, 562));
            segs.push((true, if (word >> i) & 1 == 1 { 1687 } else { 562 }));
        }
        segs.push((false, 562));
        segs.push((true, 20000));
        segs
    }

    fn find<'a>(a: &'a [Annotation], kind: &str) -> &'a Annotation {
        a.iter().find(|x| x.kind == kind).unwrap()
    }

    #[test]
    fn nec_address_and_command() {
        let a = decode_nec(&trace(&nec_frame([0x04, 0xFB, 0x08, 0xF7])));
        let addr = find(&a, "address");
        assert_eq!(addr.text, "0x04");
        assert_eq!(addr.value, Some(4));
        assert!(!addr.error);
        let cmd = find(&a, "command");
        assert_eq!(cmd.text, "0x08");
        assert_eq!(cmd.value, Some(8));
        assert!(!cmd.error);
    }

    #[test]
    fn nec_repeat() {
        let segs = vec![(false, 9000), (true, 2250), (false, 562), (true, 96000)];
        let a = decode_nec(&trace(&segs));
        assert!(a.iter().any(|x| x.kind == "command" && x.text == "repeat"));
        assert!(a.iter().all(|x| !x.error));
    }

    #[test]
    fn nec_wrong_command_complement_is_error() {
        let a = decode_nec(&trace(&nec_frame([0x04, 0xFB, 0x08, 0x00])));
        let cmd = find(&a, "command");
        assert_eq!(cmd.value, Some(8));
        assert!(cmd.error);
    }

    /// Frame RC5: bit logici 1 = basso→alto a metà bit; il ricevitore è il negato.
    fn rc5_frame(toggle: u8, addr: u8, cmd: u8) -> Vec<(bool, usize)> {
        let mut bits = vec![1u8, 1, toggle];
        bits.extend((0..5).rev().map(|i| (addr >> i) & 1));
        bits.extend((0..6).rev().map(|i| (cmd >> i) & 1));
        let mut segs = Vec::new();
        for b in bits {
            let (first, second) = if b == 1 { (false, true) } else { (true, false) };
            segs.push((!first, 889));
            segs.push((!second, 889));
        }
        segs.push((true, 20000));
        segs
    }

    #[test]
    fn rc5_address_command_toggle() {
        let a = decode_rc5(&trace(&rc5_frame(1, 5, 12)));
        assert_eq!(find(&a, "bit").value, Some(1));
        let addr = find(&a, "address");
        assert_eq!(addr.value, Some(5));
        assert_eq!(addr.text, "0x05");
        let cmd = find(&a, "command");
        assert_eq!(cmd.value, Some(12));
        assert_eq!(cmd.text, "0x0C");
    }
}
