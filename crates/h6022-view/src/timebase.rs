//! Scale dei tempi e delle tensioni e scelta dei parametri di acquisizione. Solo calcolo.

use h6022_usb::{ChannelMode, SampleRate, VoltageRange};

pub const DIVISIONS_X: f32 = 10.0;
pub const DIVISIONS_Y: f32 = 8.0;
/// Campioni massimi per canale in un frame.
pub const MAX_RECORD: usize = 1_000_000;
pub const MIN_RECORD: usize = 1_000;

const TIME_STEPS: &[f64] = &[
    1e-7, 2e-7, 5e-7, 1e-6, 2e-6, 5e-6, 1e-5, 2e-5, 5e-5, 1e-4, 2e-4, 5e-4, 1e-3, 2e-3, 5e-3, 1e-2, 2e-2, 5e-2,
    1e-1, 2e-1, 5e-1, 1.0, 2.0, 5.0, 10.0,
];

const VOLT_STEPS: &[f32] = &[0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0];

/// Secondi per divisione in sequenza 1-2-5, da 100 ns a 10 s.
pub fn time_steps() -> &'static [f64] {
    TIME_STEPS
}

/// Volt per divisione in sequenza 1-2-5, da 10 mV a 5 V (all'ingresso dell'oscilloscopio).
pub fn volt_steps() -> &'static [f32] {
    VOLT_STEPS
}

/// Valore adiacente a `current` in `steps`: `dir` > 0 il successivo, < 0 il precedente.
/// Se `current` non è nella lista, parte dal valore più vicino. Si ferma agli estremi.
pub fn step<T: Copy + PartialOrd + Into<f64>>(steps: &[T], current: T, dir: i32) -> T {
    if steps.is_empty() {
        return current;
    }
    let idx = match steps.iter().position(|&s| s == current) {
        Some(i) => i,
        None => {
            let target: f64 = current.into();
            let distance = |s: &T| {
                let value: f64 = (*s).into();
                (value - target).abs()
            };
            steps
                .iter()
                .enumerate()
                .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)))
                .map_or(0, |(i, _)| i)
        }
    };
    let last = steps.len() - 1;
    let next = match dir {
        d if d > 0 => (idx + 1).min(last),
        d if d < 0 => idx.saturating_sub(1),
        _ => idx,
    };
    steps[next]
}

/// Frequenza massima affidabile senza perdita di dati: 30 MS/s a un canale, 16 MS/s a due.
pub fn max_reliable_rate(mode: ChannelMode) -> SampleRate {
    let limit_hz = match mode {
        ChannelMode::Single => 30_000_000,
        ChannelMode::Dual => 16_000_000,
    };
    SampleRate::all()
        .iter()
        .rev()
        .copied()
        .find(|rate| rate.hz() <= limit_hz)
        .unwrap_or(SampleRate::all()[0])
}

/// Frequenza e lunghezza del record per riempire 10 divisioni a `time_per_div`.
/// Sceglie la frequenza più alta non oltre `max_reliable_rate` con record <= MAX_RECORD.
/// Il record è almeno MIN_RECORD. Se nessuna frequenza basta, usa la più bassa e MAX_RECORD.
pub fn acquisition_for(time_per_div: f64, mode: ChannelMode) -> (SampleRate, usize) {
    let limit_hz = max_reliable_rate(mode).hz();
    let candidates = SampleRate::all()
        .iter()
        .rev()
        .copied()
        .filter(|rate| rate.hz() <= limit_hz);
    for rate in candidates {
        let needed = (10.0 * time_per_div * f64::from(rate.hz())).ceil() as usize;
        let record = needed.max(MIN_RECORD);
        if record <= MAX_RECORD {
            return (rate, record);
        }
    }
    (SampleRate::all()[0], MAX_RECORD)
}

/// La portata più sensibile che contiene ±4 divisioni attorno allo zero spostato di `offset_div`,
/// all'ingresso dell'oscilloscopio (volt della sonda divisi per `probe`). Se nessuna basta, V5.
pub fn range_for(volts_per_div: f32, offset_div: f32, probe: f32) -> VoltageRange {
    let coverage = (4.0 + offset_div.abs()) * volts_per_div / probe;
    [VoltageRange::V0_5, VoltageRange::V1, VoltageRange::V2_5, VoltageRange::V5]
        .into_iter()
        .find(|range| range.half_span_volts() >= coverage)
        .unwrap_or(VoltageRange::V5)
}

/// "100 ns", "2 µs", "50 ms", "1 s"; negativi con il segno meno. Tre cifre significative al massimo.
pub fn format_time(seconds: f64) -> String {
    scaled(
        seconds,
        "0 s",
        &[(1e-6, 1e9, "ns"), (1e-3, 1e6, "µs"), (1.0, 1e3, "ms"), (f64::INFINITY, 1.0, "s")],
    )
}

/// "10 mV", "1.5 V". Tre cifre significative al massimo.
pub fn format_volts(volts: f32) -> String {
    scaled(f64::from(volts), "0 V", &[(1.0, 1e3, "mV"), (f64::INFINITY, 1.0, "V")])
}

/// "50 Hz", "11 kHz", "1.2 MHz". Tre cifre significative al massimo.
pub fn format_hz(hz: f64) -> String {
    scaled(hz, "0 Hz", &[(1e3, 1.0, "Hz"), (1e6, 1e-3, "kHz"), (f64::INFINITY, 1e-6, "MHz")])
}

/// Valore con segno nell'unità della prima soglia non superata. `tiers` è (soglia, fattore, unità)
/// in ordine di soglia crescente; l'ultima soglia è infinita.
fn scaled(value: f64, zero: &str, tiers: &[(f64, f64, &str)]) -> String {
    let magnitude = round_significant(value.abs());
    if magnitude == 0.0 {
        return zero.to_string();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let &(_, factor, unit) = tiers
        .iter()
        .find(|tier| magnitude < tier.0)
        .unwrap_or(&tiers[tiers.len() - 1]);
    format!("{sign}{} {unit}", significant(magnitude * factor))
}

/// Arrotonda a tre cifre significative prima di scegliere l'unità: 0,9996 V diventa 1 V, non 1000 mV.
fn round_significant(value: f64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let scale = 10f64.powi(2 - value.log10().floor() as i32);
    (value * scale).round() / scale
}

/// Tre cifre significative, senza zeri finali. `value` positivo e non nullo.
fn significant(value: f64) -> String {
    let exponent = value.log10().floor() as i32;
    let decimals = (2 - exponent).max(0) as usize;
    let text = format!("{:.*}", decimals, value);
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_ms_per_div_picks_highest_reliable_rate() {
        let (rate, record) = acquisition_for(1e-3, ChannelMode::Single);
        assert!(record <= MAX_RECORD);
        assert!(record >= MIN_RECORD);
        assert_eq!(rate.hz(), 30_000_000);
    }

    #[test]
    fn dual_channel_is_capped_at_16_mhz() {
        let (rate, record) = acquisition_for(1e-3, ChannelMode::Dual);
        assert!(record <= MAX_RECORD);
        assert_eq!(rate.hz(), 16_000_000);
    }

    #[test]
    fn ten_s_per_div_falls_back_to_lowest_rate() {
        let (rate, record) = acquisition_for(10.0, ChannelMode::Single);
        assert_eq!(rate.hz(), 20_000);
        assert_eq!(record, MAX_RECORD);
    }

    #[test]
    fn max_reliable_rate_per_mode() {
        assert_eq!(max_reliable_rate(ChannelMode::Single).hz(), 30_000_000);
        assert_eq!(max_reliable_rate(ChannelMode::Dual).hz(), 16_000_000);
    }

    #[test]
    fn step_stays_at_the_ends() {
        let steps = time_steps();
        let first = steps[0];
        let last = steps[steps.len() - 1];
        assert_eq!(step(steps, first, -1), first);
        assert_eq!(step(steps, last, 1), last);
        let volts = volt_steps();
        assert_eq!(step(volts, volts[0], -1), volts[0]);
        assert_eq!(step(volts, volts[volts.len() - 1], 1), volts[volts.len() - 1]);
    }

    #[test]
    fn step_moves_to_neighbours_and_from_nearest() {
        assert_eq!(step(&[1.0, 2.0, 5.0], 2.0, 1), 5.0);
        assert_eq!(step(&[1.0, 2.0, 5.0], 2.0, -1), 1.0);
        // 3.0 non è nella lista: parte dal più vicino (2.0) e va al successivo.
        assert_eq!(step(&[1.0, 2.0, 5.0], 3.0, 1), 5.0);
    }

    #[test]
    fn range_for_picks_most_sensitive_range_that_fits() {
        assert_eq!(range_for(0.1, 0.0, 1.0), VoltageRange::V0_5);
        assert_eq!(range_for(1.0, 0.0, 10.0), VoltageRange::V0_5);
        assert_eq!(range_for(1.0, 0.0, 1.0), VoltageRange::V5);
        assert_eq!(range_for(10.0, 0.0, 1.0), VoltageRange::V5);
    }

    #[test]
    fn format_time_uses_units_and_three_digits() {
        assert_eq!(format_time(2e-6), "2 µs");
        assert_eq!(format_time(100e-9), "100 ns");
        assert_eq!(format_time(0.05), "50 ms");
        assert_eq!(format_time(1.0), "1 s");
        assert_eq!(format_time(1.23456), "1.23 s");
        assert_eq!(format_time(-1.5e-3), "-1.5 ms");
        assert_eq!(format_time(0.0), "0 s");
    }

    #[test]
    fn rounding_moves_to_the_next_unit() {
        assert_eq!(format_volts(0.9996), "1 V");
        assert_eq!(format_time(999.7e-9), "1 µs");
    }

    #[test]
    fn format_volts_and_hz() {
        assert_eq!(format_volts(0.01), "10 mV");
        assert_eq!(format_volts(1.5), "1.5 V");
        assert_eq!(format_volts(0.0), "0 V");
        assert_eq!(format_hz(50.0), "50 Hz");
        assert_eq!(format_hz(11_000.0), "11 kHz");
        assert_eq!(format_hz(1_200_000.0), "1.2 MHz");
        assert_eq!(format_hz(0.0), "0 Hz");
    }
}
