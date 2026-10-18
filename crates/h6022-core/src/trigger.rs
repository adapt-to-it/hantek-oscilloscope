//! Trigger software in streaming.

use h6022_usb::Channel;

use crate::types::{Level, Polarity, Slope, TriggerConfig, TriggerKind, WidthCondition};

/// Rivelatore con stato: i blocchi successivi sono un unico segnale continuo.
pub struct Trigger {
    config: TriggerConfig,
    sample_rate: f64,
    /// Holdoff in campioni: ceil(holdoff * sample_rate).
    holdoff_samples: u64,
    state: State,
}

/// Stato che attraversa i blocchi. Azzerato da `reset`.
#[derive(Default)]
struct State {
    crossing: Crossing,
    /// Inizio dell'impulso in corso (PulseWidth).
    pulse_start: Option<u64>,
    /// `Some(reached)` mentre il segnale è in un'escursione del runt.
    runt: Option<bool>,
    /// Ultimo campione dentro la fascia (Window).
    prev_inside: Option<bool>,
    /// Ultimo esito del pattern (Pattern).
    prev_match: Option<bool>,
    /// Indice dell'ultimo fronte, o del primo campione dopo reset (Timeout).
    last_edge: Option<u64>,
    timeout_fired: bool,
    /// Indice dell'ultimo trigger emesso, per l'holdoff.
    last_trigger: Option<u64>,
}

/// Armo dei fronti con isteresi: il fronte di salita si arma sotto level - metà isteresi,
/// quello di discesa sopra level + metà isteresi.
#[derive(Default)]
struct Crossing {
    armed_rise: bool,
    armed_fall: bool,
}

impl Crossing {
    fn step(&mut self, v: f32, level: f32, half: f32) -> Option<Slope> {
        if v < level - half {
            self.armed_rise = true;
        }
        if v > level + half {
            self.armed_fall = true;
        }
        if self.armed_rise && v >= level {
            self.armed_rise = false;
            return Some(Slope::Rising);
        }
        if self.armed_fall && v <= level {
            self.armed_fall = false;
            return Some(Slope::Falling);
        }
        None
    }
}

impl Trigger {
    pub fn new(config: TriggerConfig, sample_rate: f64) -> Trigger {
        let holdoff_samples = (config.holdoff * sample_rate).ceil().max(0.0) as u64;
        Trigger {
            config,
            sample_rate,
            holdoff_samples,
            state: State::default(),
        }
    }

    /// Esamina un blocco. `ch1`/`ch2` hanno la stessa lunghezza (ch2 vuoto se assente);
    /// `first_sample` è l'indice assoluto del primo campione.
    /// Restituisce gli indici assoluti dei campioni di trigger, in ordine, rispettando l'holdoff.
    pub fn feed(&mut self, ch1: &[f32], ch2: &[f32], first_sample: u64) -> Vec<u64> {
        let mut out = Vec::new();
        if matches!(self.config.kind, TriggerKind::Pattern { .. }) {
            // Il pattern usa sempre entrambi i canali: senza ch2 non si valuta nulla.
            let n = ch1.len().min(ch2.len());
            for i in 0..n {
                self.step_pattern(first_sample + i as u64, ch1[i], ch2[i], &mut out);
            }
            return out;
        }
        let src = match self.config.source {
            Channel::Ch1 => ch1,
            Channel::Ch2 => ch2,
        };
        // Percorso diretto per il fronte, il caso più comune: niente match per campione.
        if let TriggerKind::Edge { slope } = self.config.kind {
            let level = self.config.level;
            let half = self.config.hysteresis.max(0.0) / 2.0;
            for (i, &v) in src.iter().enumerate() {
                if let Some(ev) = self.state.crossing.step(v, level, half)
                    && slope_matches(slope, ev) {
                        self.emit(first_sample + i as u64, &mut out);
                    }
            }
            return out;
        }
        for (i, &v) in src.iter().enumerate() {
            self.step(first_sample + i as u64, v, &mut out);
        }
        out
    }

    /// Dimentica lo stato (dopo una riconfigurazione).
    pub fn reset(&mut self) {
        self.state = State::default();
    }

    pub fn config(&self) -> &TriggerConfig {
        &self.config
    }

    fn step(&mut self, idx: u64, v: f32, out: &mut Vec<u64>) {
        let level = self.config.level;
        let half = self.config.hysteresis.max(0.0) / 2.0;
        match self.config.kind {
            TriggerKind::Edge { slope } => {
                if let Some(ev) = self.state.crossing.step(v, level, half)
                    && slope_matches(slope, ev) {
                        self.emit(idx, out);
                    }
            }
            TriggerKind::PulseWidth { polarity, condition } => {
                if let Some(ev) = self.state.crossing.step(v, level, half) {
                    let (start, end) = match polarity {
                        Polarity::Positive => (Slope::Rising, Slope::Falling),
                        Polarity::Negative => (Slope::Falling, Slope::Rising),
                    };
                    if ev == start {
                        self.state.pulse_start = Some(idx);
                    } else if ev == end
                        && let Some(s) = self.state.pulse_start.take() {
                            let width = (idx - s) as f64 / self.sample_rate;
                            if width_ok(condition, width) {
                                self.emit(idx, out);
                            }
                        }
                }
            }
            TriggerKind::Runt { polarity, low, high } => {
                let (start, reach, end) = match polarity {
                    Polarity::Positive => (v > low, v >= high, v < low),
                    Polarity::Negative => (v < high, v <= low, v > high),
                };
                let runt = self.state.runt;
                let next = match runt {
                    None => {
                        if start {
                            Some(reach)
                        } else {
                            None
                        }
                    }
                    Some(reached) => {
                        let reached = reached || reach;
                        if end {
                            if !reached {
                                self.emit(idx, out);
                            }
                            None
                        } else {
                            Some(reached)
                        }
                    }
                };
                self.state.runt = next;
            }
            TriggerKind::Window { low, high, enter } => {
                let inside = v >= low && v <= high;
                let fire = match self.state.prev_inside.replace(inside) {
                    Some(prev) if enter => !prev && inside,
                    Some(prev) => prev && !inside,
                    None => false,
                };
                if fire {
                    self.emit(idx, out);
                }
            }
            TriggerKind::Timeout { slope, seconds } => {
                let ev = self.state.crossing.step(v, level, half);
                let last = *self.state.last_edge.get_or_insert(idx);
                let elapsed = idx.saturating_sub(last) as f64;
                if !self.state.timeout_fired && elapsed >= seconds * self.sample_rate {
                    self.state.timeout_fired = true;
                    self.emit(idx, out);
                }
                if let Some(ev) = ev
                    && slope_matches(slope, ev) {
                        self.state.last_edge = Some(idx);
                        self.state.timeout_fired = false;
                    }
            }
            TriggerKind::Pattern { .. } => {}
        }
    }

    fn step_pattern(&mut self, idx: u64, a: f32, b: f32, out: &mut Vec<u64>) {
        if let TriggerKind::Pattern { ch1, ch2 } = self.config.kind {
            let level = self.config.level;
            let matched = level_ok(ch1, a, level) && level_ok(ch2, b, level);
            // Scatta quando il pattern diventa vero, non finché resta vero.
            if self.state.prev_match.replace(matched) == Some(false) && matched {
                self.emit(idx, out);
            }
        }
    }

    /// Emette `idx` se l'holdoff dall'ultimo trigger è scaduto.
    fn emit(&mut self, idx: u64, out: &mut Vec<u64>) {
        if let Some(last) = self.state.last_trigger
            && idx < last + self.holdoff_samples {
                return;
            }
        self.state.last_trigger = Some(idx);
        out.push(idx);
    }
}

fn slope_matches(slope: Slope, ev: Slope) -> bool {
    match slope {
        Slope::Either => true,
        s => s == ev,
    }
}

fn width_ok(condition: WidthCondition, d: f64) -> bool {
    match condition {
        WidthCondition::LessThan(x) => d < x,
        WidthCondition::GreaterThan(x) => d > x,
        WidthCondition::Within(a, b) => d >= a && d <= b,
        WidthCondition::Outside(a, b) => d < a || d > b,
    }
}

fn level_ok(level: Level, v: f32, threshold: f32) -> bool {
    match level {
        Level::High => v > threshold,
        Level::Low => v < threshold,
        Level::DontCare => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 1000.0; // un campione = 1 ms

    fn cfg(source: Channel, kind: TriggerKind, level: f32, hysteresis: f32, holdoff: f64) -> TriggerConfig {
        TriggerConfig {
            source,
            kind,
            level,
            hysteresis,
            holdoff,
        }
    }

    fn edge(slope: Slope, level: f32, hysteresis: f32, holdoff: f64) -> Trigger {
        Trigger::new(
            cfg(Channel::Ch1, TriggerKind::Edge { slope }, level, hysteresis, holdoff),
            SR,
        )
    }

    fn run(t: &mut Trigger, samples: &[f32]) -> Vec<u64> {
        t.feed(samples, &[], 0)
    }

    #[test]
    fn edge_rising_fires_on_crossing_up() {
        let mut t = edge(Slope::Rising, 0.5, 0.0, 0.0);
        assert_eq!(run(&mut t, &[0.0, 0.0, 1.0, 1.0, 0.0, 1.0]), vec![2, 5]);
    }

    #[test]
    fn edge_falling_fires_on_crossing_down() {
        let mut t = edge(Slope::Falling, 0.5, 0.0, 0.0);
        assert_eq!(run(&mut t, &[1.0, 1.0, 0.0, 0.0, 1.0, 0.0]), vec![2, 5]);
    }

    #[test]
    fn edge_either_fires_on_both_slopes() {
        let mut t = edge(Slope::Either, 0.5, 0.0, 0.0);
        assert_eq!(run(&mut t, &[0.0, 1.0, 0.0]), vec![1, 2]);
    }

    #[test]
    fn edge_hysteresis_ignores_noise_around_level() {
        // Isteresi 0.4: il fronte di salita si arma solo sotto -0.2.
        let mut t = edge(Slope::Rising, 0.0, 0.4, 0.0);
        assert_eq!(run(&mut t, &[0.05, -0.1, 0.05, -0.1, 0.05]), Vec::<u64>::new());
        assert_eq!(t.feed(&[-0.3, 0.05], &[], 5), vec![6]);
    }

    #[test]
    fn edge_fires_once_when_crossing_is_split_across_blocks() {
        let mut t = edge(Slope::Rising, 0.5, 0.0, 0.0);
        assert_eq!(t.feed(&[0.0, 0.0], &[], 0), Vec::<u64>::new());
        assert_eq!(t.feed(&[1.0, 1.0], &[], 2), vec![2]);
        assert_eq!(t.feed(&[1.0], &[], 4), Vec::<u64>::new());
    }

    #[test]
    fn edge_on_ch2_without_ch2_never_fires() {
        let mut t = Trigger::new(
            cfg(Channel::Ch2, TriggerKind::Edge { slope: Slope::Rising }, 0.5, 0.0, 0.0),
            SR,
        );
        assert_eq!(t.feed(&[0.0, 1.0], &[], 0), Vec::<u64>::new());
        assert_eq!(t.feed(&[0.0, 0.0], &[0.0, 1.0], 2), vec![3]);
    }

    #[test]
    fn pulse_width_positive_within_and_outside_condition() {
        let kind = TriggerKind::PulseWidth {
            polarity: Polarity::Positive,
            condition: WidthCondition::Within(0.002, 0.004),
        };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        // Impulso da 3 ms, inizio al campione 2, fine al 5.
        assert_eq!(run(&mut t, &[0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]), vec![5]);

        // Impulso da 5 ms: fuori da Within(0.002, 0.004) ma dentro Outside, quindi scatta.
        let mut t = Trigger::new(
            cfg(Channel::Ch1, TriggerKind::PulseWidth { polarity: Polarity::Positive, condition: WidthCondition::Outside(0.002, 0.004) }, 0.5, 0.0, 0.0),
            SR,
        );
        assert_eq!(run(&mut t, &[0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]), vec![6]);

        let mut t = Trigger::new(
            cfg(Channel::Ch1, TriggerKind::PulseWidth { polarity: Polarity::Positive, condition: WidthCondition::LessThan(0.003) }, 0.5, 0.0, 0.0),
            SR,
        );
        assert_eq!(run(&mut t, &[0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]), Vec::<u64>::new());
    }

    #[test]
    fn pulse_width_negative_ends_on_rising_edge() {
        let kind = TriggerKind::PulseWidth {
            polarity: Polarity::Negative,
            condition: WidthCondition::Within(0.001, 0.002),
        };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        // Impulso sotto il livello da 1 campione alla discesa in 1, risalita in 3: 2 ms.
        assert_eq!(run(&mut t, &[1.0, 0.0, 0.0, 1.0, 1.0]), vec![3]);
    }

    #[test]
    fn pulse_width_split_across_blocks() {
        let kind = TriggerKind::PulseWidth {
            polarity: Polarity::Positive,
            condition: WidthCondition::Within(0.002, 0.004),
        };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        assert_eq!(t.feed(&[0.0, 0.0, 1.0], &[], 0), Vec::<u64>::new());
        assert_eq!(t.feed(&[1.0, 1.0, 0.0], &[], 3), vec![5]);
    }

    #[test]
    fn runt_positive_fires_when_pulse_returns_below_low() {
        let kind = TriggerKind::Runt { polarity: Polarity::Positive, low: 0.3, high: 0.8 };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.0, 0.0, 0.0), SR);
        // Primo impulso non arriva a high (rientro a 2), secondo lo supera (nessun trigger).
        assert_eq!(run(&mut t, &[0.0, 0.5, 0.2, 0.0, 0.5, 0.9, 0.2]), vec![2]);
    }

    #[test]
    fn runt_negative_fires_when_dip_returns_above_high() {
        let kind = TriggerKind::Runt { polarity: Polarity::Negative, low: 0.2, high: 0.7 };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.0, 0.0, 0.0), SR);
        // Primo dip non arriva a low (rientro a 2), secondo lo supera (nessun trigger).
        assert_eq!(run(&mut t, &[1.0, 0.5, 0.8, 1.0, 0.5, 0.1, 0.8]), vec![2]);
    }

    #[test]
    fn window_enter_and_exit() {
        let samples = [0.1, 0.5, 0.9, 0.5, 0.1, 0.5];
        let kind = |enter| TriggerKind::Window { low: 0.3, high: 0.7, enter };

        let mut t = Trigger::new(cfg(Channel::Ch1, kind(true), 0.0, 0.0, 0.0), SR);
        assert_eq!(run(&mut t, &samples), vec![1, 3, 5]);

        let mut t = Trigger::new(cfg(Channel::Ch1, kind(false), 0.0, 0.0, 0.0), SR);
        assert_eq!(run(&mut t, &samples), vec![2, 4]);

        // Partire già dentro non conta come ingresso.
        let mut t = Trigger::new(cfg(Channel::Ch1, kind(true), 0.0, 0.0, 0.0), SR);
        assert_eq!(run(&mut t, &[0.5, 0.5]), Vec::<u64>::new());
    }

    #[test]
    fn timeout_fires_once_then_rearms_on_next_edge() {
        // Timeout di 5 campioni.
        let kind = TriggerKind::Timeout { slope: Slope::Rising, seconds: 0.005 };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        // Fronte di salita a 1, timeout a 6; fronte a 8, timeout a 13.
        let samples = [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        assert_eq!(run(&mut t, &samples), vec![6, 13]);
    }

    #[test]
    fn timeout_restarts_from_first_sample_after_reset() {
        let kind = TriggerKind::Timeout { slope: Slope::Rising, seconds: 0.005 };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        assert_eq!(t.feed(&[0.0, 0.0, 0.0], &[], 0), Vec::<u64>::new());
        t.reset();
        assert_eq!(t.feed(&[0.0; 6], &[], 100), vec![105]);
    }

    #[test]
    fn pattern_fires_when_combined_condition_becomes_true() {
        let kind = TriggerKind::Pattern { ch1: Level::High, ch2: Level::Low };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        let ch1 = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let ch2 = [1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        assert_eq!(t.feed(&ch1, &ch2, 0), vec![1, 5]);
    }

    #[test]
    fn pattern_without_ch2_never_fires() {
        let kind = TriggerKind::Pattern { ch1: Level::High, ch2: Level::DontCare };
        let mut t = Trigger::new(cfg(Channel::Ch1, kind, 0.5, 0.0, 0.0), SR);
        assert_eq!(t.feed(&[0.0, 1.0], &[], 0), Vec::<u64>::new());
    }

    #[test]
    fn holdoff_suppresses_triggers_closer_than_holdoff_samples() {
        // Holdoff 2.5 ms a 1 kHz: ceil -> 3 campioni.
        let mut t = edge(Slope::Rising, 0.5, 0.0, 0.0025);
        assert_eq!(run(&mut t, &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]), vec![1, 5]);
    }

    #[test]
    fn holdoff_applies_across_blocks() {
        let mut t = edge(Slope::Rising, 0.5, 0.0, 0.0025);
        assert_eq!(t.feed(&[0.0, 1.0, 0.0], &[], 0), vec![1]);
        assert_eq!(t.feed(&[1.0, 0.0, 1.0], &[], 3), vec![5]);
    }
}
