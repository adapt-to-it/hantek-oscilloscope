//! Bus 1-Wire (Dallas/Maxim), velocità standard.
//!
//! Tempi: reset basso >= 480 µs; presenza basso 60..240 µs dopo il reset;
//! bit 1 basso 1..15 µs; bit 0 basso 60..120 µs. Byte LSB per primo.

use crate::{Annotation, LogicTrace};

/// Stato del decoder dopo il reset.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nessun reset visto, o presenza assente: i tratti bassi sono ignorati.
    Idle,
    /// Reset visto: il prossimo tratto basso deve essere la presenza.
    AwaitPresence,
    /// Presenza confermata: i tratti bassi sono bit.
    Bits,
}

/// Nome dei comandi ROM noti.
fn rom_command(byte: u8) -> Option<&'static str> {
    match byte {
        0x33 => Some("READ ROM"),
        0x55 => Some("MATCH ROM"),
        0xCC => Some("SKIP ROM"),
        0xF0 => Some("SEARCH ROM"),
        0xEC => Some("ALARM SEARCH"),
        _ => None,
    }
}

/// Annotazioni "reset", "presence" (o "error" "nessuna presenza"), "data" per byte con testo "0x33",
/// "command" per il primo byte dopo il reset se è un comando ROM noto
/// (0x33 READ ROM, 0x55 MATCH ROM, 0xCC SKIP ROM, 0xF0 SEARCH ROM, 0xEC ALARM SEARCH).
pub fn decode(trace: &LogicTrace) -> Vec<Annotation> {
    // Tratti bassi misurati in µs: reset >= 480; presenza 60..=240; bit 1 in 1..=15; bit 0 in 60..=120.
    // Un tratto basso fuori da questi intervalli è "error" "tempo non valido" e scarta il byte in corso.
    let mut out = Vec::new();
    let us_per_sample = 1e6 / trace.sample_rate;
    let mut state = State::Idle;
    let mut reset_end = 0usize;
    let mut byte: u8 = 0;
    let mut nbits: u32 = 0;
    let mut byte_start = 0usize;
    let mut first_byte = false;

    for (start, end, level) in trace.runs() {
        // I tratti alti separano solo i tratti bassi.
        if level {
            continue;
        }
        let us = (end - start) as f64 * us_per_sample;
        if us >= 480.0 {
            out.push(Annotation::new(trace.time(start), trace.time(end), "reset", "reset"));
            state = State::AwaitPresence;
            reset_end = end;
            byte = 0;
            nbits = 0;
            first_byte = true;
            continue;
        }
        match state {
            State::Idle => {}
            State::AwaitPresence => {
                if (60.0..=240.0).contains(&us) {
                    out.push(Annotation::new(trace.time(start), trace.time(end), "presence", "presence"));
                    state = State::Bits;
                } else {
                    out.push(
                        Annotation::new(trace.time(reset_end), trace.time(start), "error", "nessuna presenza")
                            .as_error(),
                    );
                    state = State::Idle;
                }
            }
            State::Bits => {
                let bit = if (1.0..=15.0).contains(&us) {
                    true
                } else if (60.0..=120.0).contains(&us) {
                    false
                } else {
                    out.push(Annotation::new(trace.time(start), trace.time(end), "error", "tempo non valido").as_error());
                    byte = 0;
                    nbits = 0;
                    continue;
                };
                if nbits == 0 {
                    byte_start = start;
                }
                // LSB per primo.
                byte |= u8::from(bit) << nbits;
                nbits += 1;
                if nbits == 8 {
                    let (kind, text) = match (first_byte, rom_command(byte)) {
                        (true, Some(name)) => ("command", format!("0x{:02X} {}", byte, name)),
                        _ => ("data", format!("0x{:02X}", byte)),
                    };
                    out.push(
                        Annotation::new(trace.time(byte_start), trace.time(end), kind, text)
                            .with_value(u64::from(byte)),
                    );
                    byte = 0;
                    nbits = 0;
                    first_byte = false;
                }
            }
        }
    }
    if state == State::AwaitPresence {
        // Reset senza alcun tratto basso successivo: nessun dispositivo risponde.
        out.push(Annotation::new(trace.time(reset_end), trace.time(trace.len), "error", "nessuna presenza").as_error());
    }
    out
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

    /// Reset con presenza: alto, basso 480 µs, alto, basso 100 µs (presenza), alto.
    fn reset_and_presence() -> Vec<(bool, f64)> {
        vec![(true, 10.0), (false, 480.0), (true, 70.0), (false, 100.0), (true, 400.0)]
    }

    /// Slot di scrittura di un byte, LSB per primo: bit 1 basso 6 µs, bit 0 basso 60 µs.
    fn write_byte(byte: u8, runs: &mut Vec<(bool, f64)>) {
        for k in 0..8 {
            if ((byte >> k) & 1) == 1 {
                runs.push((false, 6.0));
                runs.push((true, 64.0));
            } else {
                runs.push((false, 60.0));
                runs.push((true, 10.0));
            }
        }
    }

    #[test]
    fn reset_presence_skip_rom_then_data() {
        let mut runs = reset_and_presence();
        write_byte(0xCC, &mut runs);
        write_byte(0x44, &mut runs);
        let out = decode(&trace_from_runs(&runs));
        assert!(out.iter().any(|a| a.kind == "reset"));
        assert!(out.iter().any(|a| a.kind == "presence"));
        assert!(out.iter().any(|a| a.kind == "command" && a.text == "0xCC SKIP ROM" && a.value == Some(0xCC)));
        assert!(out.iter().any(|a| a.kind == "data" && a.text == "0x44" && a.value == Some(0x44)));
        assert!(out.iter().all(|a| !a.error));
    }

    #[test]
    fn reset_without_presence_is_error() {
        let runs = vec![(true, 10.0), (false, 480.0), (true, 500.0)];
        let out = decode(&trace_from_runs(&runs));
        assert!(out.iter().any(|a| a.kind == "reset"));
        assert!(out.iter().any(|a| a.kind == "error" && a.text == "nessuna presenza" && a.error));
    }
}
