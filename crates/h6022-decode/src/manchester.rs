//! Codifica Manchester. Convenzione IEEE 802.3: transizione basso→alto a metà bit = 1.

use std::collections::BTreeMap;

use crate::{Annotation, LogicTrace};

/// Periodo di bit dalla moda delle durate degli impulsi: la più breve frequente è mezzo bit.
/// Durate di meno di 2 campioni sono scartate; quantizzazione a 1/20 della durata minima;
/// frequente = almeno 2 occorrenze e almeno un quarto della moda.
pub fn estimate_bit_period(trace: &LogicTrace) -> Option<f64> {
    let durations: Vec<usize> = trace
        .transitions
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .filter(|&d| d >= 2)
        .collect();
    let min = *durations.iter().min()?;
    let quantum = min as f64 / 20.0;
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for &d in &durations {
        *counts.entry((d as f64 / quantum).round() as u64).or_insert(0) += 1;
    }
    let mode = counts.values().copied().max()?;
    let threshold = (mode / 4).max(2);
    // BTreeMap è in ordine crescente: la prima chiave frequente è la più breve.
    let (&half, _) = counts.iter().find(|&(_, &n)| n >= threshold)?;
    Some(2.0 * half as f64 * quantum / trace.sample_rate)
}

/// `bit_period_s` `None`: stima automatica. `ieee`: falso per la convenzione G.E. Thomas (alto→basso = 1).
/// Annotazioni "bit" per ogni bit, poi "data" per ogni gruppo di 8 bit (MSB per primo),
/// "error" dove manca la transizione a metà bit.
pub fn decode(trace: &LogicTrace, bit_period_s: Option<f64>, ieee: bool) -> Vec<Annotation> {
    let mut out = Vec::new();
    let period = match bit_period_s.or_else(|| estimate_bit_period(trace)) {
        Some(p) if p.is_finite() && p > 0.0 => p,
        _ => return out,
    };
    // Campioni per bit e mezzo bit.
    let bit = period * trace.sample_rate;
    let half = bit / 2.0;
    // Tolleranza ±25 % del mezzo bit.
    let tol = half / 4.0;
    let t = &trace.transitions;
    // Sincronia: prima coppia di transizioni distanti circa un bit intero.
    let Some(sync) = t.windows(2).position(|w| (w[1] as f64 - w[0] as f64 - bit).abs() <= tol) else {
        return out;
    };
    let at = |p: f64| trace.t0 + p / trace.sample_rate;

    // Ogni bit: (inizio, fine in campioni, valore; None se manca la transizione a metà bit).
    let mut bits: Vec<(f64, f64, Option<bool>)> = Vec::new();
    let mut expected = t[sync] as f64;
    while expected < trace.len as f64 {
        let idx = t.partition_point(|&x| (x as f64) < expected - tol);
        let found = t.get(idx).copied().filter(|&x| (x as f64) <= expected + tol);
        match found {
            Some(x) => {
                // Il livello dopo la transizione dice il valore del bit.
                let level = trace.level_at(x);
                let value = if ieee { level } else { !level };
                bits.push((x as f64 - half, x as f64 + half, Some(value)));
                // Si riallinea sulla transizione trovata, così la deriva non si accumula.
                expected = x as f64 + bit;
            }
            None => {
                bits.push((expected - half, expected + half, None));
                expected += bit;
            }
        }
    }

    for &(s, e, v) in &bits {
        let ann = Annotation::new(at(s), at(e), "bit", match v {
            Some(true) => "1",
            Some(false) => "0",
            None => "?",
        });
        out.push(match v {
            Some(b) => ann.with_value(u64::from(b)),
            None => ann,
        });
        if v.is_none() {
            out.push(Annotation::new(at(s), at(e), "error", "transizione a metà bit mancante").as_error());
        }
    }

    for chunk in bits.as_chunks::<8>().0 {
        let values: Option<Vec<bool>> = chunk.iter().map(|c| c.2).collect();
        if let Some(values) = values {
            let v = values.iter().fold(0u64, |acc, &b| (acc << 1) | u64::from(b));
            out.push(Annotation::new(at(chunk[0].0), at(chunk[7].1), "data", format!("0x{:02X}", v)).with_value(v));
        }
    }

    out.sort_by(|a, b| a.start_s.total_cmp(&b.start_s));
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

    /// Bit di un byte, MSB per primo.
    fn bits_of(byte: u8) -> Vec<bool> {
        (0..8).rev().map(|k| ((byte >> k) & 1) == 1).collect()
    }

    /// Tratti di una codifica Manchester con bit di `bit_us` µs; `ieee` per la convenzione IEEE.
    fn manchester_runs(bits: &[bool], bit_us: f64, ieee: bool) -> Vec<(bool, f64)> {
        let mut runs = Vec::new();
        for &b in bits {
            // IEEE: 1 = basso→alto. G.E. Thomas: 1 = alto→basso.
            let first = b != ieee;
            runs.push((first, bit_us / 2.0));
            runs.push((!first, bit_us / 2.0));
        }
        runs
    }

    fn data(out: &[Annotation]) -> Vec<&str> {
        out.iter().filter(|a| a.kind == "data").map(|a| a.text.as_str()).collect()
    }

    #[test]
    fn decodes_a5_at_10kbit_with_automatic_period() {
        let t = trace_from_runs(&manchester_runs(&bits_of(0xA5), 100.0, true));
        let period = estimate_bit_period(&t).unwrap();
        assert!((period - 100e-6).abs() < 1e-9);
        let out = decode(&t, None, true);
        assert_eq!(data(&out), ["0xA5"]);
        assert!(out.iter().all(|a| !a.error));
    }

    #[test]
    fn thomas_convention_inverts_bits() {
        let t = trace_from_runs(&manchester_runs(&bits_of(0xA5), 100.0, true));
        let out = decode(&t, None, false);
        assert_eq!(data(&out), ["0x5A"]);
    }
}
