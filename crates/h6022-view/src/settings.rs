//! Impostazioni di visualizzazione scelte dall'utente e loro effetto sulla configurazione del motore.

use h6022_core::{EngineConfig, Slope, TriggerConfig, TriggerKind, TriggerMode};
use h6022_usb::{AcquisitionConfig, Channel, ChannelMode, Coupling, SampleRate, VoltageRange};
use serde::{Deserialize, Serialize};

use crate::timebase::{DIVISIONS_X, acquisition_for, range_for};
use crate::trace::Viewport;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSettings {
    pub enabled: bool,
    pub volts_per_div: f32,
    pub offset_div: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewSettings {
    pub channels: [ChannelSettings; 2],
    /// Secondi per divisione.
    pub time_per_div: f64,
    /// Spostamento orizzontale in secondi (positivo = la vista va a destra).
    pub horizontal_offset: f64,
}

impl Default for ViewSettings {
    fn default() -> Self {
        let ch = ChannelSettings { enabled: true, volts_per_div: 1.0, offset_div: 0.0 };
        ViewSettings {
            channels: [ch, ChannelSettings { offset_div: -3.0, ..ch }],
            time_per_div: 100e-6,
            horizontal_offset: 0.0,
        }
    }
}

impl ViewSettings {
    /// Le 10 divisioni visibili, centrate su `horizontal_offset`.
    pub fn viewport(&self, columns: usize) -> Viewport {
        let half = f64::from(DIVISIONS_X) / 2.0 * self.time_per_div;
        Viewport {
            t_start: self.horizontal_offset - half,
            t_end: self.horizontal_offset + half,
            columns,
        }
    }

    /// Configurazione iniziale del motore: trigger Auto sul fronte di salita di CH1 a 1,5 V,
    /// accoppiamento DC, sonde x1, poi `apply`.
    pub fn initial_config(&self) -> EngineConfig {
        let mut config = EngineConfig {
            acquisition: AcquisitionConfig {
                sample_rate: SampleRate::nearest(1e6),
                mode: ChannelMode::Dual,
                range: [VoltageRange::V5; 2],
                coupling: [Coupling::Dc; 2],
            },
            trigger: TriggerConfig {
                source: Channel::Ch1,
                kind: TriggerKind::Edge { slope: Slope::Rising },
                level: 1.5,
                hysteresis: 0.1,
                holdoff: 0.0,
            },
            mode: TriggerMode::Auto,
            record_length: 1000,
            pretrigger: 0.5,
            probe: [1.0, 1.0],
        };
        self.apply(&mut config);
        config
    }

    /// Frequenza, numero di canali, record e portate che servono a questa vista.
    pub fn apply(&self, config: &mut EngineConfig) {
        let mode = if self.channels[1].enabled { ChannelMode::Dual } else { ChannelMode::Single };
        let (rate, record) = acquisition_for(self.time_per_div, mode);
        config.acquisition.sample_rate = rate;
        config.acquisition.mode = mode;
        config.record_length = record;
        for ch in 0..2 {
            let view = self.channels[ch];
            config.acquisition.range[ch] = range_for(view.volts_per_div, view.offset_div, config.probe[ch]);
        }
    }
}
