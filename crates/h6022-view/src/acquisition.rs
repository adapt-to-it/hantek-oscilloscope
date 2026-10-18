//! Impostazioni di acquisizione e trigger scambiate con il frontend (serde camelCase).
//! Frequenza, record e portate restano derivati dalla vista (`settings::ViewSettings::apply`).

use h6022_core::{
    EngineConfig, Level, Polarity, Slope, TriggerConfig, TriggerKind, TriggerMode, WidthCondition,
};
use h6022_usb::{Channel, Coupling};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeName {
    Auto,
    Normal,
    Single,
    Roll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlopeName {
    Rising,
    Falling,
    Either,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolarityName {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LevelName {
    High,
    Low,
    Any,
}

/// Condizione sulla durata dell'impulso, in secondi. `b` serve solo a `within` e `outside`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidthSpec {
    /// "less", "greater", "within" o "outside".
    pub op: String,
    pub a: f64,
    pub b: f64,
}

/// Tipo di trigger, con tag `kind`: `{"kind":"edge","slope":"rising"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TriggerSpec {
    Edge { slope: SlopeName },
    PulseWidth { polarity: PolarityName, width: WidthSpec },
    Runt { polarity: PolarityName, low: f32, high: f32 },
    Window { low: f32, high: f32, enter: bool },
    Timeout { slope: SlopeName, seconds: f64 },
    Pattern { ch1: LevelName, ch2: LevelName },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquisitionSettings {
    pub mode: ModeName,
    /// 1 o 10 per canale.
    pub probe: [f32; 2],
    /// true = accoppiamento AC.
    pub ac: [bool; 2],
    /// Frazione 0..1 del frame prima del trigger.
    pub pretrigger: f32,
    /// 1 o 2.
    pub trigger_source: u8,
    pub trigger: TriggerSpec,
    pub trigger_level: f32,
    pub hysteresis: f32,
    pub holdoff: f64,
}

impl AcquisitionSettings {
    /// Legge le impostazioni dalla configurazione del motore.
    pub fn from_config(config: &EngineConfig) -> Self {
        let trigger = config.trigger;
        Self {
            mode: mode_name(config.mode),
            probe: config.probe,
            ac: config.acquisition.coupling.map(|c| c == Coupling::Ac),
            pretrigger: config.pretrigger,
            trigger_source: match trigger.source {
                Channel::Ch1 => 1,
                Channel::Ch2 => 2,
            },
            trigger: trigger_spec(trigger.kind),
            trigger_level: trigger.level,
            hysteresis: trigger.hysteresis,
            holdoff: trigger.holdoff,
        }
    }

    /// Scrive le impostazioni in `config`. Errore in italiano se un valore non è valido:
    /// sonda diversa da 1 o 10, pretrigger fuori 0..=1, sorgente diversa da 1 o 2,
    /// operatore di `WidthSpec` sconosciuto, valori non finiti o negativi dove servono positivi.
    /// In caso di errore `config` resta invariato.
    pub fn apply(&self, config: &mut EngineConfig) -> Result<(), String> {
        for p in self.probe {
            if p != 1.0 && p != 10.0 {
                return Err(format!("sonda non valida: {p} (ammesse 1 e 10)"));
            }
        }
        if !(0.0..=1.0).contains(&self.pretrigger) {
            return Err(format!("pretrigger fuori da 0..=1: {}", self.pretrigger));
        }
        let source = match self.trigger_source {
            1 => Channel::Ch1,
            2 => Channel::Ch2,
            other => return Err(format!("sorgente di trigger non valida: {other} (ammesse 1 e 2)")),
        };
        check_finite("livello di trigger", f64::from(self.trigger_level))?;
        check_non_negative("isteresi", f64::from(self.hysteresis))?;
        check_non_negative("holdoff", self.holdoff)?;
        let kind = trigger_kind(&self.trigger)?;

        let mut next = *config;
        next.acquisition.coupling = self.ac.map(|ac| if ac { Coupling::Ac } else { Coupling::Dc });
        next.probe = self.probe;
        next.pretrigger = self.pretrigger;
        next.mode = mode_from(self.mode);
        next.trigger = TriggerConfig {
            source,
            kind,
            level: self.trigger_level,
            hysteresis: self.hysteresis,
            holdoff: self.holdoff,
        };
        *config = next;
        Ok(())
    }
}

fn mode_name(mode: TriggerMode) -> ModeName {
    match mode {
        TriggerMode::Auto => ModeName::Auto,
        TriggerMode::Normal => ModeName::Normal,
        TriggerMode::Single => ModeName::Single,
        TriggerMode::Roll => ModeName::Roll,
    }
}

fn mode_from(mode: ModeName) -> TriggerMode {
    match mode {
        ModeName::Auto => TriggerMode::Auto,
        ModeName::Normal => TriggerMode::Normal,
        ModeName::Single => TriggerMode::Single,
        ModeName::Roll => TriggerMode::Roll,
    }
}

fn slope_name(slope: Slope) -> SlopeName {
    match slope {
        Slope::Rising => SlopeName::Rising,
        Slope::Falling => SlopeName::Falling,
        Slope::Either => SlopeName::Either,
    }
}

fn slope_from(slope: SlopeName) -> Slope {
    match slope {
        SlopeName::Rising => Slope::Rising,
        SlopeName::Falling => Slope::Falling,
        SlopeName::Either => Slope::Either,
    }
}

fn polarity_name(polarity: Polarity) -> PolarityName {
    match polarity {
        Polarity::Positive => PolarityName::Positive,
        Polarity::Negative => PolarityName::Negative,
    }
}

fn polarity_from(polarity: PolarityName) -> Polarity {
    match polarity {
        PolarityName::Positive => Polarity::Positive,
        PolarityName::Negative => Polarity::Negative,
    }
}

fn level_name(level: Level) -> LevelName {
    match level {
        Level::High => LevelName::High,
        Level::Low => LevelName::Low,
        Level::DontCare => LevelName::Any,
    }
}

fn level_from(level: LevelName) -> Level {
    match level {
        LevelName::High => Level::High,
        LevelName::Low => Level::Low,
        LevelName::Any => Level::DontCare,
    }
}

fn trigger_spec(kind: TriggerKind) -> TriggerSpec {
    match kind {
        TriggerKind::Edge { slope } => TriggerSpec::Edge { slope: slope_name(slope) },
        TriggerKind::PulseWidth { polarity, condition } => TriggerSpec::PulseWidth {
            polarity: polarity_name(polarity),
            width: width_spec(condition),
        },
        TriggerKind::Runt { polarity, low, high } => {
            TriggerSpec::Runt { polarity: polarity_name(polarity), low, high }
        }
        TriggerKind::Window { low, high, enter } => TriggerSpec::Window { low, high, enter },
        TriggerKind::Timeout { slope, seconds } => {
            TriggerSpec::Timeout { slope: slope_name(slope), seconds }
        }
        TriggerKind::Pattern { ch1, ch2 } => {
            TriggerSpec::Pattern { ch1: level_name(ch1), ch2: level_name(ch2) }
        }
    }
}

fn trigger_kind(spec: &TriggerSpec) -> Result<TriggerKind, String> {
    Ok(match spec {
        TriggerSpec::Edge { slope } => TriggerKind::Edge { slope: slope_from(*slope) },
        TriggerSpec::PulseWidth { polarity, width } => TriggerKind::PulseWidth {
            polarity: polarity_from(*polarity),
            condition: width_condition(width)?,
        },
        TriggerSpec::Runt { polarity, low, high } => {
            check_band("runt", *low, *high)?;
            TriggerKind::Runt { polarity: polarity_from(*polarity), low: *low, high: *high }
        }
        TriggerSpec::Window { low, high, enter } => {
            check_band("finestra", *low, *high)?;
            TriggerKind::Window { low: *low, high: *high, enter: *enter }
        }
        TriggerSpec::Timeout { slope, seconds } => {
            check_non_negative("timeout", *seconds)?;
            TriggerKind::Timeout { slope: slope_from(*slope), seconds: *seconds }
        }
        TriggerSpec::Pattern { ch1, ch2 } => {
            TriggerKind::Pattern { ch1: level_from(*ch1), ch2: level_from(*ch2) }
        }
    })
}

/// `less` e `greater` usano solo `a`: in lettura `b` vale 0.
fn width_spec(condition: WidthCondition) -> WidthSpec {
    let (op, a, b) = match condition {
        WidthCondition::LessThan(a) => ("less", a, 0.0),
        WidthCondition::GreaterThan(a) => ("greater", a, 0.0),
        WidthCondition::Within(a, b) => ("within", a, b),
        WidthCondition::Outside(a, b) => ("outside", a, b),
    };
    WidthSpec { op: op.to_string(), a, b }
}

fn width_condition(width: &WidthSpec) -> Result<WidthCondition, String> {
    check_non_negative("durata a", width.a)?;
    check_non_negative("durata b", width.b)?;
    match width.op.as_str() {
        "less" => Ok(WidthCondition::LessThan(width.a)),
        "greater" => Ok(WidthCondition::GreaterThan(width.a)),
        "within" | "outside" if width.b <= width.a => Err(format!(
            "durata {}: b ({}) deve essere maggiore di a ({})",
            width.op, width.b, width.a
        )),
        "within" => Ok(WidthCondition::Within(width.a, width.b)),
        "outside" => Ok(WidthCondition::Outside(width.a, width.b)),
        other => Err(format!("operatore di durata sconosciuto: {other}")),
    }
}

fn check_finite(nome: &str, valore: f64) -> Result<(), String> {
    if valore.is_finite() {
        Ok(())
    } else {
        Err(format!("{nome} non finito: {valore}"))
    }
}

fn check_non_negative(nome: &str, valore: f64) -> Result<(), String> {
    if valore.is_finite() && valore >= 0.0 {
        Ok(())
    } else {
        Err(format!("{nome} deve essere finito e non negativo: {valore}"))
    }
}

/// Fascia `low..high` in volt: entrambi finiti e `high` maggiore di `low`.
fn check_band(nome: &str, low: f32, high: f32) -> Result<(), String> {
    check_finite(&format!("{nome}: livello basso"), f64::from(low))?;
    check_finite(&format!("{nome}: livello alto"), f64::from(high))?;
    if high <= low {
        return Err(format!("{nome}: alto ({high}) deve essere maggiore di basso ({low})"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ViewSettings;

    fn base() -> EngineConfig {
        ViewSettings::default().initial_config()
    }

    fn settings_with(trigger: TriggerSpec) -> AcquisitionSettings {
        AcquisitionSettings {
            mode: ModeName::Normal,
            probe: [10.0, 1.0],
            ac: [true, false],
            pretrigger: 0.25,
            trigger_source: 2,
            trigger,
            trigger_level: 0.75,
            hysteresis: 0.05,
            holdoff: 2e-4,
        }
    }

    fn all_variants() -> Vec<TriggerSpec> {
        vec![
            TriggerSpec::Edge { slope: SlopeName::Falling },
            TriggerSpec::PulseWidth {
                polarity: PolarityName::Negative,
                width: WidthSpec { op: "less".into(), a: 1e-6, b: 0.0 },
            },
            TriggerSpec::PulseWidth {
                polarity: PolarityName::Positive,
                width: WidthSpec { op: "greater".into(), a: 2e-6, b: 0.0 },
            },
            TriggerSpec::PulseWidth {
                polarity: PolarityName::Positive,
                width: WidthSpec { op: "within".into(), a: 1e-6, b: 5e-6 },
            },
            TriggerSpec::PulseWidth {
                polarity: PolarityName::Negative,
                width: WidthSpec { op: "outside".into(), a: 1e-6, b: 5e-6 },
            },
            TriggerSpec::Runt { polarity: PolarityName::Positive, low: -0.5, high: 1.2 },
            TriggerSpec::Window { low: 0.2, high: 1.8, enter: false },
            TriggerSpec::Timeout { slope: SlopeName::Either, seconds: 0.01 },
            TriggerSpec::Pattern { ch1: LevelName::High, ch2: LevelName::Any },
        ]
    }

    #[test]
    fn andata_e_ritorno_per_ogni_variante() {
        for trigger in all_variants() {
            let settings = settings_with(trigger.clone());
            let mut config = base();
            settings.apply(&mut config).unwrap();
            assert_eq!(AcquisitionSettings::from_config(&config), settings, "{trigger:?}");
        }
    }

    #[test]
    fn ogni_errore_lascia_la_config_invariata() {
        let casi: Vec<Box<dyn Fn(&mut AcquisitionSettings)>> = vec![
            Box::new(|s| s.probe[0] = 5.0),
            Box::new(|s| s.probe[1] = f32::NAN),
            Box::new(|s| s.pretrigger = 1.5),
            Box::new(|s| s.pretrigger = -0.1),
            Box::new(|s| s.pretrigger = f32::NAN),
            Box::new(|s| s.trigger_source = 0),
            Box::new(|s| s.trigger_source = 3),
            Box::new(|s| s.trigger_level = f32::INFINITY),
            Box::new(|s| s.hysteresis = -0.1),
            Box::new(|s| s.hysteresis = f32::NAN),
            Box::new(|s| s.holdoff = -1.0),
            Box::new(|s| s.holdoff = f64::INFINITY),
            Box::new(|s| {
                s.trigger = TriggerSpec::PulseWidth {
                    polarity: PolarityName::Positive,
                    width: WidthSpec { op: "bogus".into(), a: 1e-6, b: 0.0 },
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::PulseWidth {
                    polarity: PolarityName::Positive,
                    width: WidthSpec { op: "less".into(), a: f64::NAN, b: 0.0 },
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::PulseWidth {
                    polarity: PolarityName::Positive,
                    width: WidthSpec { op: "greater".into(), a: -1.0, b: 0.0 },
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::PulseWidth {
                    polarity: PolarityName::Positive,
                    width: WidthSpec { op: "within".into(), a: 2e-6, b: 2e-6 },
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::PulseWidth {
                    polarity: PolarityName::Positive,
                    width: WidthSpec { op: "outside".into(), a: 2e-6, b: 1e-6 },
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::Runt {
                    polarity: PolarityName::Positive,
                    low: 1.0,
                    high: 1.0,
                }
            }),
            Box::new(|s| {
                s.trigger = TriggerSpec::Runt {
                    polarity: PolarityName::Negative,
                    low: f32::NAN,
                    high: 1.0,
                }
            }),
            Box::new(|s| s.trigger = TriggerSpec::Window { low: 1.8, high: 0.2, enter: true }),
            Box::new(|s| {
                s.trigger = TriggerSpec::Timeout { slope: SlopeName::Rising, seconds: -1.0 }
            }),
        ];
        for caso in &casi {
            let mut settings = settings_with(TriggerSpec::Edge { slope: SlopeName::Rising });
            caso(&mut settings);
            let mut config = base();
            let prima = config;
            let err = settings.apply(&mut config).expect_err("doveva essere rifiutato");
            assert!(!err.is_empty());
            assert_eq!(config, prima, "config cambiata dopo l'errore: {err}");
        }
    }

    #[test]
    fn messaggio_di_errore_in_italiano() {
        let mut settings = settings_with(TriggerSpec::Edge { slope: SlopeName::Rising });
        settings.probe[0] = 5.0;
        let err = settings.apply(&mut base()).unwrap_err();
        assert!(err.starts_with("sonda non valida"), "{err}");
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn edge_json_uses_kind_tag() {
        let spec = TriggerSpec::Edge { slope: SlopeName::Rising };
        assert_eq!(serde_json::to_string(&spec).unwrap(), r#"{"kind":"edge","slope":"rising"}"#);
        let pw: TriggerSpec =
            serde_json::from_str(r#"{"kind":"pulseWidth","polarity":"positive","width":{"op":"greater","a":0.001,"b":0}}"#)
                .unwrap();
        assert!(matches!(pw, TriggerSpec::PulseWidth { .. }));
    }
}
