//! UART asincrona, livello di riposo alto (TTL). `inverted` per RS-232 dopo un traslatore assente.

use crate::{Annotation, LogicTrace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UartConfig {
    /// `None`: stima automatica con `estimate_baud`.
    pub baud: Option<f64>,
    /// 5..=9.
    pub data_bits: u8,
    pub parity: Parity,
    /// 1.0, 1.5 o 2.0.
    pub stop_bits: f32,
    /// LSB per primo, come lo standard.
    pub lsb_first: bool,
    pub inverted: bool,
}

impl Default for UartConfig {
    fn default() -> Self {
        UartConfig { baud: None, data_bits: 8, parity: Parity::None, stop_bits: 1.0, lsb_first: true, inverted: false }
    }
}

/// Velocità standard usate da `estimate_baud` per arrotondare.
pub const STANDARD_BAUDS: [f64; 14] = [
    300.0, 1200.0, 2400.0, 4800.0, 9600.0, 14400.0, 19200.0, 28800.0, 38400.0, 57600.0, 115200.0,
    230400.0, 460800.0, 921600.0,
];

/// Baud dalla durata dell'impulso più breve; se entro il 5 % di una velocità standard, restituisce quella.
/// `None` con meno di 2 transizioni.
pub fn estimate_baud(trace: &LogicTrace) -> Option<f64> {
    if trace.transitions.len() < 2 {
        return None;
    }
    // Impulsi di meno di 2 campioni sono glitch: non entrano nella stima.
    let min = trace
        .transitions
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .filter(|&d| d >= 2)
        .min()?;
    let raw = trace.sample_rate / min as f64;
    let standard = STANDARD_BAUDS.iter().copied().find(|&s| (raw - s).abs() <= 0.05 * s);
    Some(standard.unwrap_or(raw))
}

/// Annotazioni "start", "data" (testo "0x41 'A'", value = byte), "parity", "stop", "error"
/// ("framing" se lo stop è basso, "parity" se la parità non torna).
pub fn decode(trace: &LogicTrace, config: &UartConfig) -> Vec<Annotation> {
    let mut out = Vec::new();
    // Configurazione non valida: nessuna annotazione.
    if !(5..=9).contains(&config.data_bits) {
        return out;
    }
    let baud = match config.baud.or_else(|| estimate_baud(trace)) {
        Some(b) if b.is_finite() && b > 0.0 => b,
        _ => return out,
    };
    // Campioni per bit (non intero in generale).
    let bit = trace.sample_rate / baud;
    let data_bits = usize::from(config.data_bits);
    let parity_bits = usize::from(config.parity != Parity::None);
    // Indice del primo bit di stop: start (0), dati, parità.
    let stop_idx = 1 + data_bits + parity_bits;
    let at = |p: f64| trace.t0 + p / trace.sample_rate;
    // Livello logico: con `inverted` il riposo è basso sulla linea.
    let logic = |idx: usize| trace.level_at(idx) ^ config.inverted;
    let mut pos = 0usize;
    while let Some(t) = trace.next_transition(pos) {
        if logic(t) {
            // Fronte di salita: la linea torna al riposo, nessun frame.
            pos = t + 1;
            continue;
        }
        let start = t as f64;
        // Inizio del bit k (k = 0 è lo start).
        let edge = |k: usize| start + k as f64 * bit;
        // Livello logico al centro del bit k.
        let level = |k: usize| logic((edge(k) + bit / 2.0) as usize);
        let frame_end = edge(stop_idx) + f64::from(config.stop_bits) * bit;
        if edge(stop_idx) + bit / 2.0 >= trace.len as f64 {
            // Frame incompleto alla fine della traccia.
            break;
        }

        let mut value: u64 = 0;
        let mut ones = 0usize;
        for k in 0..data_bits {
            if level(1 + k) {
                ones += 1;
                let shift = if config.lsb_first { k } else { data_bits - 1 - k };
                value |= 1u64 << shift;
            }
        }
        out.push(Annotation::new(at(edge(0)), at(edge(1)), "start", "start"));
        out.push(Annotation::new(at(edge(1)), at(edge(1 + data_bits)), "data", byte_text(value)).with_value(value));

        if config.parity != Parity::None {
            let p_idx = 1 + data_bits;
            let p = level(p_idx);
            // Pari: uni di dati e parità in numero pari; dispari: in numero dispari.
            let ok = if config.parity == Parity::Even { (ones % 2 == 1) == p } else { ones.is_multiple_of(2) == p };
            let span = (at(edge(p_idx)), at(edge(p_idx + 1)));
            out.push(Annotation::new(span.0, span.1, "parity", if p { "1" } else { "0" }).with_value(u64::from(p)));
            if !ok {
                out.push(Annotation::new(span.0, span.1, "error", "parity").as_error());
            }
        }

        let stop_ok = level(stop_idx);
        let stop_span = (at(edge(stop_idx)), at(frame_end));
        out.push(Annotation::new(stop_span.0, stop_span.1, "stop", "stop"));
        if !stop_ok {
            out.push(Annotation::new(stop_span.0, stop_span.1, "error", "framing").as_error());
        }

        // Dopo lo stop si riprende dalla transizione successiva.
        pos = (frame_end.floor() as usize).max(t + 1);
    }
    out
}

/// Testo di un byte: carattere se stampabile ASCII, altrimenti solo esadecimale.
fn byte_text(value: u64) -> String {
    if (0x20..=0x7E).contains(&value) {
        format!("0x{:02X} '{}'", value, value as u8 as char)
    } else {
        format!("0x{:02X}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traccia a 1 MHz (un campione per µs) da tratti (livello, durata in µs).
    fn trace_from_runs(runs: &[(bool, f64)]) -> LogicTrace {
        let mut transitions = Vec::new();
        let mut cum: f64 = 0.0;
        for (i, &(level, us)) in runs.iter().enumerate() {
            if i > 0 && level != runs[i - 1].0 {
                transitions.push(cum.round() as usize);
            }
            cum += us;
        }
        LogicTrace::from_transitions(1e6, cum.round() as usize, runs[0].0, transitions)
    }

    /// Tratti di linea per una sequenza di bit logici, con 2 bit di riposo ai lati.
    fn uart_runs(frame: &[bool], baud: f64, inverted: bool) -> Vec<(bool, f64)> {
        let bit_us = 1e6 / baud;
        let idle = !inverted;
        let mut runs = vec![(idle, 2.0 * bit_us)];
        runs.extend(frame.iter().map(|&b| (b ^ inverted, bit_us)));
        runs.push((idle, 2.0 * bit_us));
        runs
    }

    /// Bit logici di un frame 8N1 (LSB per primo) con parità e stop opzionali.
    fn frame(byte: u8, parity: Option<bool>, stop: bool) -> Vec<bool> {
        let mut bits = vec![false];
        bits.extend((0..8).map(|k| ((byte >> k) & 1) == 1));
        bits.extend(parity);
        bits.push(stop);
        bits
    }

    fn cfg(baud: Option<f64>) -> UartConfig {
        UartConfig { baud, ..UartConfig::default() }
    }

    #[test]
    fn decodes_hi_8n1_with_automatic_baud() {
        let mut bits = frame(b'H', None, true);
        bits.extend(frame(b'i', None, true));
        let t = trace_from_runs(&uart_runs(&bits, 9600.0, false));
        let out = decode(&t, &cfg(None));
        let texts: Vec<&str> = out.iter().filter(|a| a.kind == "data").map(|a| a.text.as_str()).collect();
        assert_eq!(texts, ["0x48 'H'", "0x69 'i'"]);
        assert!(out.iter().all(|a| !a.error));
    }

    #[test]
    fn estimates_9600_baud() {
        let t = trace_from_runs(&uart_runs(&frame(b'H', None, true), 9600.0, false));
        assert_eq!(estimate_baud(&t), Some(9600.0));
    }

    #[test]
    fn wrong_parity_is_reported() {
        // 'H' ha due bit a 1 nei dati: con parità pari il bit corretto è 0.
        let config = UartConfig { baud: Some(9600.0), parity: Parity::Even, ..UartConfig::default() };
        let good = trace_from_runs(&uart_runs(&frame(b'H', Some(false), true), 9600.0, false));
        assert!(decode(&good, &config).iter().all(|a| !a.error));
        let bad = trace_from_runs(&uart_runs(&frame(b'H', Some(true), true), 9600.0, false));
        assert!(decode(&bad, &config).iter().any(|a| a.kind == "error" && a.text == "parity"));
    }

    #[test]
    fn low_stop_bit_is_framing_error() {
        let t = trace_from_runs(&uart_runs(&frame(b'H', None, false), 9600.0, false));
        let out = decode(&t, &cfg(Some(9600.0)));
        assert!(out.iter().any(|a| a.kind == "error" && a.text == "framing"));
    }

    #[test]
    fn inverted_line_decodes_after_negation() {
        let t = trace_from_runs(&uart_runs(&frame(b'H', None, true), 9600.0, true));
        let config = UartConfig { baud: Some(9600.0), inverted: true, ..UartConfig::default() };
        let out = decode(&t, &config);
        let texts: Vec<&str> = out.iter().filter(|a| a.kind == "data").map(|a| a.text.as_str()).collect();
        assert_eq!(texts, ["0x48 'H'"]);
    }
}
