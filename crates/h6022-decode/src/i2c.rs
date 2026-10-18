//! Bus I2C su due canali: SCL e SDA.

use crate::{Annotation, LogicTrace};

/// Stato del decoder: fuori da una transazione, oppure dentro un byte.
/// `edges` conta i fronti di salita di SCL già letti (0..=8; 8 = prossimo è l'ack).
#[derive(Clone, Copy)]
enum St {
    Idle,
    Bits { addr: bool, edges: u8, value: u8, first: usize },
}

/// Annotazioni "start" (anche ripetuto), "address" (testo "0x50 W" o "0x50 R", value = indirizzo 7 bit),
/// "data" (testo "0xA5"), "ack", "nack", "stop". Campiona SDA sul fronte di salita di SCL.
/// Start: SDA scende con SCL alto. Stop: SDA sale con SCL alto.
/// Le due tracce devono avere la stessa frequenza e lo stesso numero di campioni.
pub fn decode(scl: &LogicTrace, sda: &LogicTrace) -> Vec<Annotation> {
    if scl.sample_rate != sda.sample_rate || scl.len != sda.len {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut st = St::Idle;
    let (mut i, mut j) = (0, 0);
    loop {
        let e_scl = scl.transitions.get(i).copied();
        let e_sda = sda.transitions.get(j).copied();
        let idx = match (e_scl, e_sda) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };
        let scl_edge = e_scl == Some(idx);
        let sda_edge = e_sda == Some(idx);
        if scl_edge {
            i += 1;
        }
        if sda_edge {
            j += 1;
        }
        let scl_high = scl.level_at(idx);
        let sda_high = sda.level_at(idx);
        let t = scl.time(idx);

        if sda_edge && !scl_edge && scl_high {
            // Cambio di SDA con SCL alto: start o stop.
            if sda_high {
                out.push(Annotation::new(t, t, "stop", "stop"));
                st = St::Idle;
            } else {
                let text = match st {
                    St::Idle => "start",
                    St::Bits { .. } => "start ripetuto",
                };
                out.push(Annotation::new(t, t, "start", text));
                st = St::Bits { addr: true, edges: 0, value: 0, first: idx };
            }
        } else if scl_edge && scl_high {
            // Fronte di salita di SCL: si legge SDA.
            if let St::Bits { addr, edges, value, first } = st {
                st = if edges < 8 {
                    let value = (value << 1) | u8::from(sda_high);
                    let first = if edges == 0 { idx } else { first };
                    if edges == 7 {
                        let (s, e) = (scl.time(first), t);
                        if addr {
                            let rw = if value & 1 == 1 { "R" } else { "W" };
                            let text = format!("0x{:02X} {}", value >> 1, rw);
                            out.push(
                                Annotation::new(s, e, "address", text).with_value(u64::from(value >> 1)),
                            );
                        } else {
                            let text = format!("0x{:02X}", value);
                            out.push(Annotation::new(s, e, "data", text).with_value(u64::from(value)));
                        }
                    }
                    St::Bits { addr, edges: edges + 1, value, first }
                } else {
                    // Nono fronte: ack se SDA basso.
                    let kind = if sda_high { "nack" } else { "ack" };
                    let end = scl.time(scl.next_transition(idx + 1).unwrap_or(scl.len));
                    out.push(Annotation::new(t, end, kind, kind));
                    St::Bits { addr: false, edges: 0, value: 0, first: 0 }
                };
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 10_000_000.0;

    /// Livelli di SCL e SDA campione per campione. Ogni bit dura 12 campioni:
    /// 4 con SCL basso (SDA cambia lì), 8 con SCL alto.
    struct Wire {
        scl: Vec<bool>,
        sda: Vec<bool>,
    }

    impl Wire {
        fn new() -> Self {
            Wire { scl: vec![true; 4], sda: vec![true; 4] }
        }

        fn push(&mut self, scl: bool, sda: bool, n: usize) {
            for _ in 0..n {
                self.scl.push(scl);
                self.sda.push(sda);
            }
        }

        /// Start (anche ripetuto): SDA scende con SCL alto.
        fn start(&mut self) {
            self.push(false, true, 4);
            self.push(true, true, 4);
            self.push(true, false, 4);
            self.push(false, false, 4);
        }

        /// Stop: SDA sale con SCL alto.
        fn stop(&mut self) {
            self.push(false, false, 4);
            self.push(true, false, 4);
            self.push(true, true, 4);
        }

        fn bit(&mut self, b: bool) {
            self.push(false, b, 4);
            self.push(true, b, 8);
        }

        fn byte(&mut self, v: u8) {
            for k in (0..8).rev() {
                self.bit((v >> k) & 1 == 1);
            }
        }

        fn traces(&self) -> (LogicTrace, LogicTrace) {
            (from_levels(&self.scl), from_levels(&self.sda))
        }
    }

    fn from_levels(levels: &[bool]) -> LogicTrace {
        let transitions = (1..levels.len()).filter(|&i| levels[i] != levels[i - 1]).collect();
        LogicTrace::from_transitions(FS, levels.len(), levels[0], transitions)
    }

    fn kinds(anns: &[Annotation]) -> Vec<&'static str> {
        anns.iter().map(|a| a.kind).collect()
    }

    #[test]
    fn scrittura_con_ack_e_stop() {
        let mut w = Wire::new();
        w.start();
        w.byte(0xA0);
        w.bit(false);
        w.byte(0x00);
        w.bit(false);
        w.byte(0xA5);
        w.bit(false);
        w.stop();
        let (scl, sda) = w.traces();
        let anns = decode(&scl, &sda);
        assert_eq!(kinds(&anns), ["start", "address", "ack", "data", "ack", "data", "ack", "stop"]);
        assert_eq!(anns[1].text, "0x50 W");
        assert_eq!(anns[1].value, Some(0x50));
        assert_eq!(anns[3].text, "0x00");
        assert_eq!(anns[5].text, "0xA5");
        assert_eq!(anns[5].value, Some(0xA5));
        assert!(anns.iter().all(|a| !a.error));
    }

    #[test]
    fn lettura_con_nack_finale() {
        let mut w = Wire::new();
        w.start();
        w.byte(0xA1);
        w.bit(false);
        w.byte(0x5A);
        w.bit(true);
        w.stop();
        let (scl, sda) = w.traces();
        let anns = decode(&scl, &sda);
        assert_eq!(kinds(&anns), ["start", "address", "ack", "data", "nack", "stop"]);
        assert_eq!(anns[1].text, "0x50 R");
        assert_eq!(anns[3].text, "0x5A");
    }

    #[test]
    fn start_ripetuto() {
        let mut w = Wire::new();
        w.start();
        w.byte(0xA0);
        w.bit(false);
        w.byte(0x00);
        w.bit(false);
        w.start();
        w.byte(0xA1);
        w.bit(true);
        w.stop();
        let (scl, sda) = w.traces();
        let anns = decode(&scl, &sda);
        assert_eq!(
            kinds(&anns),
            ["start", "address", "ack", "data", "ack", "start", "address", "nack", "stop"]
        );
        assert_eq!(anns[5].text, "start ripetuto");
        assert_eq!(anns[6].text, "0x50 R");
    }

    #[test]
    fn tracce_incompatibili_danno_vuoto() {
        let mut w = Wire::new();
        w.start();
        w.stop();
        let (scl, sda) = w.traces();
        let other = LogicTrace::from_transitions(FS / 2.0, sda.len, sda.initial, sda.transitions.clone());
        assert!(decode(&scl, &other).is_empty());
    }
}
