//! CAN 2.0A e 2.0B su un solo filo (CAN-H rispetto a massa, oppure CAN-RX di un transceiver).
//! Recessivo = alto per CAN-RX; con `inverted` recessivo = basso (CAN-H).

use crate::{Annotation, LogicTrace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanConfig {
    pub bitrate: f64,
    pub inverted: bool,
}

/// Annotazioni per frame: "start", "id" (testo "0x123" o "0x1ABCDE12 ext", value = id),
/// "command" ("RTR" per i remote frame), "data" per ogni byte, "crc" (error se il CRC-15 non torna),
/// "ack" o "nack", "stop". Rimuove i bit di stuffing; sei bit uguali danno "error" "stuff".
pub fn decode(trace: &LogicTrace, config: &CanConfig) -> Vec<Annotation> {
    let bit = trace.sample_rate / config.bitrate;
    let logic = |idx: usize| trace.level_at(idx) != config.inverted;
    let mut out = Vec::new();
    let mut cursor = 0;
    for (k, &t) in trace.transitions.iter().enumerate() {
        // SOF: fronte verso dominante preceduto da almeno 11 bit recessivi.
        if t < cursor || logic(t) {
            continue;
        }
        let prev = if k == 0 { 0 } else { trace.transitions[k - 1] };
        if ((t - prev) as f64) < 11.0 * bit {
            continue;
        }
        let mut fr = Frame {
            trace,
            inverted: config.inverted,
            bit,
            pos: t as f64,
            last: None,
            run: 0,
            covered: Vec::new(),
            out: Vec::new(),
        };
        fr.frame();
        cursor = (fr.pos.ceil() as usize).max(t + 1);
        out.extend(fr.out);
    }
    out
}

/// CRC-15 CAN (polinomio 0x4599) su una sequenza di bit.
pub fn crc15(bits: &[bool]) -> u16 {
    let mut reg: u16 = 0;
    for &bit in bits {
        let nxt = bit ^ ((reg >> 14) & 1 == 1);
        reg = (reg << 1) & 0x7FFF;
        if nxt {
            reg ^= 0x4599;
        }
    }
    reg
}

/// Bit campionato al 70 % del suo intervallo; `start` ed `end` in campioni.
#[derive(Clone, Copy)]
struct Bit {
    value: bool,
    start: f64,
    end: f64,
}

/// Lettura di un frame a partire dal fronte di SOF.
struct Frame<'a> {
    trace: &'a LogicTrace,
    inverted: bool,
    /// Campioni per bit.
    bit: f64,
    /// Inizio del prossimo bit, in campioni.
    pos: f64,
    /// Ultimo bit destuffato e lunghezza della sua serie.
    last: Option<bool>,
    run: u32,
    /// Bit destuffati da SOF in poi, base del CRC.
    covered: Vec<bool>,
    out: Vec<Annotation>,
}

impl<'a> Frame<'a> {
    fn logic(&self, idx: usize) -> bool {
        self.trace.level_at(idx) != self.inverted
    }

    fn secs(&self, x: f64) -> f64 {
        self.trace.t0 + x / self.trace.sample_rate
    }

    fn ann(&self, s: f64, e: f64, kind: &'static str, text: impl Into<String>) -> Annotation {
        Annotation::new(self.secs(s), self.secs(e), kind, text)
    }

    /// Campiona il bit corrente e fissa l'inizio del successivo, risincronizzando
    /// sul fronte recessivo→dominante più vicino al confine nominale.
    fn sample(&mut self) -> Option<Bit> {
        let start = self.pos;
        let end = start + self.bit;
        let at = start + 0.7 * self.bit;
        if at >= self.trace.len as f64 {
            return None;
        }
        let value = self.logic(at as usize);
        self.pos = self.resync(end);
        Some(Bit { value, start, end })
    }

    /// Fronte verso dominante entro mezzo bit dal confine nominale, altrimenti il confine stesso.
    fn resync(&self, nominal: f64) -> f64 {
        let lo = (nominal - 0.5 * self.bit).ceil() as usize;
        let hi = (nominal + 0.5 * self.bit).ceil() as usize;
        let mut t = self.trace.next_transition(lo);
        while let Some(x) = t {
            if x >= hi {
                break;
            }
            if !self.logic(x) {
                return x as f64;
            }
            t = self.trace.next_transition(x + 1);
        }
        nominal
    }

    fn accept(&mut self, v: bool) {
        if self.last == Some(v) {
            self.run += 1;
        } else {
            self.run = 1;
        }
        self.last = Some(v);
    }

    /// Legge un bit; con `stuffed` scarta il bit di stuffing dopo cinque bit uguali.
    /// `None` alla fine della traccia o dopo un errore di stuffing (già annotato).
    fn read(&mut self, stuffed: bool) -> Option<Bit> {
        let mut b = self.sample()?;
        if stuffed {
            if self.run == 5 {
                if self.last == Some(b.value) {
                    let a = self.ann(b.start, b.end, "error", "stuff").as_error();
                    self.out.push(a);
                    return None;
                }
                self.accept(b.value);
                b = self.sample()?;
            }
            self.accept(b.value);
        }
        Some(b)
    }

    /// Legge `n` bit MSB per primo. Restituisce valore, inizio e fine del campo.
    fn field(&mut self, n: usize, stuffed: bool) -> Option<(u64, f64, f64)> {
        let start = self.pos;
        let mut value = 0u64;
        let mut end = start;
        for _ in 0..n {
            let b = self.read(stuffed)?;
            value = (value << 1) | u64::from(b.value);
            end = b.end;
            if stuffed {
                self.covered.push(b.value);
            }
        }
        Some((value, start, end))
    }

    /// Un frame da SOF a fine EOF. Al ritorno `pos` è il primo bit dopo il frame.
    fn frame(&mut self) {
        let Some((_, s0, s1)) = self.field(1, true) else { return };
        let a = self.ann(s0, s1, "start", "SOF");
        self.out.push(a);

        let Some((base, id_s, base_e)) = self.field(11, true) else { return };
        // Dopo l'ID base: RTR nei frame standard, SRR in quelli estesi.
        let Some((slot, slot_s, slot_e)) = self.field(1, true) else { return };
        let Some((ide, _, _)) = self.field(1, true) else { return };
        let (id, rtr, id_e, rtr_span) = if ide == 1 {
            let Some((ext, _, ext_e)) = self.field(18, true) else { return };
            let Some((rtr, rs, re)) = self.field(1, true) else { return };
            let Some(_) = self.field(1, true) else { return }; // r1
            ((base << 18) | ext, rtr == 1, ext_e, (rs, re))
        } else {
            (base, slot == 1, base_e, (slot_s, slot_e))
        };
        let text = if ide == 1 { format!("0x{id:X} ext") } else { format!("0x{id:X}") };
        let a = self.ann(id_s, id_e, "id", text).with_value(id);
        self.out.push(a);
        if rtr {
            let a = self.ann(rtr_span.0, rtr_span.1, "command", "RTR");
            self.out.push(a);
        }

        let Some(_) = self.field(1, true) else { return }; // r0
        let Some((dlc, _, _)) = self.field(4, true) else { return };
        let len = if rtr { 0 } else { (dlc as usize).min(8) };
        for _ in 0..len {
            let Some((b, s, e)) = self.field(8, true) else { return };
            let a = self.ann(s, e, "data", format!("0x{b:02X}")).with_value(b);
            self.out.push(a);
        }

        // Il CRC copre i bit destuffati da SOF alla fine dei dati.
        let calc = crc15(&self.covered);
        let Some((rx, cs, ce)) = self.field(15, true) else { return };
        let rx16 = rx as u16;
        let ok = rx16 == calc;
        let text = if ok {
            format!("0x{rx16:04X}")
        } else {
            format!("0x{rx16:04X} atteso 0x{calc:04X}")
        };
        let mut a = self.ann(cs, ce, "crc", text).with_value(rx);
        if !ok {
            a = a.as_error();
        }
        self.out.push(a);

        // Delimitatore CRC (può essere preceduto da uno stuffing), slot ACK, delimitatore ACK.
        let Some(_) = self.field(1, true) else { return };
        let Some((ack, as_, ae)) = self.field(1, false) else { return };
        let kind = if ack == 0 { "ack" } else { "nack" };
        let a = self.ann(as_, ae, kind, kind);
        self.out.push(a);
        let Some(_) = self.field(1, false) else { return };

        // EOF: sette bit recessivi.
        let Some((_, es, ee)) = self.field(7, false) else { return };
        let a = self.ann(es, ee, "stop", "EOF");
        self.out.push(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 10_000_000.0;
    /// Campioni per bit a 500 kbit/s.
    const BIT: usize = 20;

    fn cfg() -> CanConfig {
        CanConfig { bitrate: 500_000.0, inverted: false }
    }

    fn push_bits(v: &mut Vec<bool>, value: u32, n: usize) {
        for k in (0..n).rev() {
            v.push((value >> k) & 1 == 1);
        }
    }

    /// Inserisce un bit di stuffing dopo ogni cinque bit uguali.
    fn stuff(bits: &[bool]) -> Vec<bool> {
        let mut out = Vec::new();
        let mut last = None;
        let mut run = 0;
        for &b in bits {
            out.push(b);
            if last == Some(b) {
                run += 1;
            } else {
                run = 1;
                last = Some(b);
            }
            if run == 5 {
                out.push(!b);
                last = Some(!b);
                run = 1;
            }
        }
        out
    }

    /// Bit di un frame dati da SOF a EOF. Con `bad_crc` si inverte l'ultimo bit del CRC.
    fn frame_bits(id: u32, ext: bool, data: &[u8], bad_crc: bool) -> Vec<bool> {
        let mut b = vec![false]; // SOF
        if ext {
            push_bits(&mut b, id >> 18, 11);
            b.push(true); // SRR
            b.push(true); // IDE
            push_bits(&mut b, id & 0x3FFFF, 18);
            b.push(false); // RTR
            b.push(false); // r1
        } else {
            push_bits(&mut b, id, 11);
            b.push(false); // RTR
            b.push(false); // IDE
        }
        b.push(false); // r0
        push_bits(&mut b, data.len() as u32, 4);
        for &x in data {
            push_bits(&mut b, u32::from(x), 8);
        }
        let crc = crc15(&b) ^ u16::from(bad_crc);
        push_bits(&mut b, u32::from(crc), 15);
        let mut out = stuff(&b);
        out.extend([true, false, true]); // delimitatore CRC, ACK dominante, delimitatore ACK
        out.extend([true; 7]); // EOF
        out
    }

    fn from_levels(levels: &[bool]) -> LogicTrace {
        let transitions = (1..levels.len()).filter(|&i| levels[i] != levels[i - 1]).collect();
        LogicTrace::from_transitions(FS, levels.len(), levels[0], transitions)
    }

    /// Traccia con 12 bit recessivi di pausa, il frame e tre bit recessivi in coda.
    fn trace_of(bits: &[bool]) -> LogicTrace {
        let mut levels = vec![true; 12 * BIT];
        for &b in bits {
            levels.extend(std::iter::repeat_n(b, BIT));
        }
        levels.extend(vec![true; 3 * BIT]);
        from_levels(&levels)
    }

    #[test]
    fn frame_standard_con_dati_e_crc_valido() {
        let trace = trace_of(&frame_bits(0x123, false, &[0x11, 0x22], false));
        let anns = decode(&trace, &cfg());
        let id = anns.iter().find(|a| a.kind == "id").unwrap();
        assert_eq!(id.text, "0x123");
        assert_eq!(id.value, Some(0x123));
        let data: Vec<u64> = anns.iter().filter(|a| a.kind == "data").filter_map(|a| a.value).collect();
        assert_eq!(data, vec![0x11, 0x22]);
        let crc = anns.iter().find(|a| a.kind == "crc").unwrap();
        assert!(!crc.error);
        assert!(anns.iter().any(|a| a.kind == "ack" && !a.error));
        assert!(anns.iter().any(|a| a.kind == "stop"));
        assert!(anns.iter().all(|a| !a.error));
    }

    #[test]
    fn crc_alterato_da_errore() {
        let trace = trace_of(&frame_bits(0x123, false, &[0x11, 0x22], true));
        let anns = decode(&trace, &cfg());
        let crc = anns.iter().find(|a| a.kind == "crc").unwrap();
        assert!(crc.error);
        assert!(anns.iter().all(|a| a.text != "stuff"));
    }

    #[test]
    fn frame_esteso() {
        let trace = trace_of(&frame_bits(0x1ABC_DE12, true, &[], false));
        let anns = decode(&trace, &cfg());
        let id = anns.iter().find(|a| a.kind == "id").unwrap();
        assert_eq!(id.text, "0x1ABCDE12 ext");
        assert_eq!(id.value, Some(0x1ABC_DE12));
        assert!(anns.iter().all(|a| !a.error));
    }

    #[test]
    fn crc15_sequenza_vuota_e_zero() {
        assert_eq!(crc15(&[]), 0);
    }
}
