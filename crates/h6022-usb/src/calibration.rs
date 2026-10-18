//! Calibrazione di offset e guadagno letta dalla EEPROM.

use crate::types::{Channel, VoltageRange};

/// Byte di fabbrica di offset per la portata V0_5, V1, V2_5, V5 (canale 1; canale 2 = +1).
const FACTORY_OFFSET: [usize; 4] = [0, 6, 8, 14];
/// Byte estesi di guadagno, stesso ordine (canale 1; canale 2 = +1).
const EXT_GAIN: [usize; 4] = [32, 38, 40, 46];
/// Byte estesi di offset, stesso ordine (canale 1; canale 2 = +1).
const EXT_OFFSET: [usize; 4] = [48, 54, 56, 62];
/// Guadagni nominali per V0_5, V1, V2_5, V5.
const NOMINAL_GAIN: [f32; 4] = [1.0, 0.99, 1.01, 1.01];
/// Byte di fabbrica: lunghezza minima utile.
const FACTORY_LEN: usize = 32;

/// Correzioni per canale e portata.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    /// Offset in conteggi ADC, indicizzato da `[canale][indice portata]`.
    pub offset: [[f32; 4]; 2],
    /// Fattore di guadagno, stesso indice.
    pub gain: [[f32; 4]; 2],
}

/// Indice della portata nelle tabelle: V0_5=0, V1=1, V2_5=2, V5=3.
fn range_index(range: VoltageRange) -> usize {
    match range {
        VoltageRange::V0_5 => 0,
        VoltageRange::V1 => 1,
        VoltageRange::V2_5 => 2,
        VoltageRange::V5 => 3,
    }
}

fn channel_index(channel: Channel) -> usize {
    match channel {
        Channel::Ch1 => 0,
        Channel::Ch2 => 1,
    }
}

impl Default for Calibration {
    /// Offset nulli, guadagni nominali.
    fn default() -> Self {
        Calibration {
            offset: [[0.0; 4]; 2],
            gain: [NOMINAL_GAIN; 2],
        }
    }
}

impl Calibration {
    /// Interpreta i byte letti da `CALIBRATION_EEPROM_OFFSET`: 32 byte di fabbrica,
    /// opzionalmente seguiti dai 48 estesi. Accetta 32..=80 byte.
    pub fn from_eeprom(bytes: &[u8]) -> Calibration {
        if bytes.len() < FACTORY_LEN {
            return Calibration::default();
        }
        let mut cal = Calibration::default();
        for ch in 0..2 {
            for i in 0..4 {
                // Offset di fabbrica: sempre presente, centrato su 128.
                cal.offset[ch][i] = f32::from(bytes[FACTORY_OFFSET[i] + ch]) - 128.0;

                // I byte estesi usati solo se presenti; 0x00 e 0xFF indicano cella non scritta.
                if let Some(&b) = bytes.get(EXT_GAIN[i] + ch)
                    && b != 0 && b != 255 {
                        cal.gain[ch][i] *= 1.0 + (f32::from(b) - 128.0) / 500.0;
                    }
                if let Some(&b) = bytes.get(EXT_OFFSET[i] + ch)
                    && b != 0 && b != 255 {
                        cal.offset[ch][i] += (f32::from(b) - 128.0) / 250.0;
                    }
            }
        }
        cal
    }

    pub fn offset(&self, channel: Channel, range: VoltageRange) -> f32 {
        self.offset[channel_index(channel)][range_index(range)]
    }

    pub fn gain(&self, channel: Channel, range: VoltageRange) -> f32 {
        self.gain[channel_index(channel)][range_index(range)]
    }

    /// Converte un campione grezzo in volt all'ingresso della sonda 1:1.
    pub fn to_volts(&self, channel: Channel, range: VoltageRange, raw: u8) -> f32 {
        let centered = f32::from(raw) - 128.0 - self.offset(channel, range);
        let code = f32::from(range.code());
        centered * 5.12 * self.gain(channel, range) / (code * 128.0)
    }

    /// Converte un blocco di campioni grezzi in volt, scrivendo in `out` (che viene svuotato).
    pub fn to_volts_slice(&self, channel: Channel, range: VoltageRange, raw: &[u8], out: &mut Vec<f32>) {
        // Una tabella dei 256 codici costa meno della conversione per campione sui blocchi lunghi.
        let table: [f32; 256] = std::array::from_fn(|r| self.to_volts(channel, range, r as u8));
        out.clear();
        out.extend(raw.iter().map(|&r| table[usize::from(r)]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eeprom(len: usize) -> Vec<u8> {
        vec![128; len]
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn default_has_zero_offset_and_nominal_gain() {
        let cal = Calibration::default();
        assert_eq!(cal.offset, [[0.0; 4]; 2]);
        assert_eq!(cal.gain[0], NOMINAL_GAIN);
        assert_eq!(cal.gain[1], NOMINAL_GAIN);
    }

    #[test]
    fn short_eeprom_gives_default() {
        assert_eq!(Calibration::from_eeprom(&[0; 31]), Calibration::default());
        assert_eq!(Calibration::from_eeprom(&[]), Calibration::default());
    }

    #[test]
    fn factory_bytes_give_offsets_per_channel() {
        let mut bytes = eeprom(32);
        bytes[0] = 131; // CH1, V0_5: +3
        bytes[14] = 120; // CH1, V5: -8
        bytes[1] = 129; // CH2, V0_5: +1
        let cal = Calibration::from_eeprom(&bytes);
        assert_eq!(cal.offset(Channel::Ch1, VoltageRange::V0_5), 3.0);
        assert_eq!(cal.offset(Channel::Ch1, VoltageRange::V5), -8.0);
        assert_eq!(cal.offset(Channel::Ch2, VoltageRange::V0_5), 1.0);
        assert_eq!(cal.gain(Channel::Ch1, VoltageRange::V5), 1.01);
    }

    #[test]
    fn extended_bytes_scale_gain_and_offset_when_valid() {
        let mut bytes = eeprom(80);
        bytes[32] = 228; // CH1 gain V0_5: 1 + 100/500 = 1.2
        bytes[48] = 153; // CH1 offset V0_5: +25/250 = +0.1
        let cal = Calibration::from_eeprom(&bytes);
        assert!(approx(cal.gain(Channel::Ch1, VoltageRange::V0_5), 1.2));
        assert!(approx(cal.offset(Channel::Ch1, VoltageRange::V0_5), 0.1));
    }

    #[test]
    fn erased_extended_bytes_are_ignored() {
        let mut bytes = eeprom(80);
        bytes[32] = 0;
        bytes[33] = 255;
        bytes[48] = 255;
        let cal = Calibration::from_eeprom(&bytes);
        assert_eq!(cal, Calibration::default());
    }

    #[test]
    fn only_present_extended_bytes_are_used() {
        let mut bytes = eeprom(36);
        bytes[32] = 228; // gain CH1 V0_5 presente; il gain CH1 V1 (byte 38) manca
        let cal = Calibration::from_eeprom(&bytes);
        assert!(approx(cal.gain(Channel::Ch1, VoltageRange::V0_5), 1.2));
        assert_eq!(cal.gain(Channel::Ch1, VoltageRange::V1), 0.99);
    }

    #[test]
    fn to_volts_maps_centre_to_zero_and_scales_by_range() {
        let cal = Calibration::default();
        assert_eq!(cal.to_volts(Channel::Ch1, VoltageRange::V5, 128), 0.0);
        // Range V5 (codice 1): 127 conteggi * 5.12 * 1.01 / 128.
        let expected = 127.0 * 5.12 * 1.01 / 128.0;
        assert!(approx(cal.to_volts(Channel::Ch1, VoltageRange::V5, 255), expected));
        // Range V0_5 (codice 10): stessa escursione, dieci volte più piccola.
        let expected_small = 127.0 * 5.12 / (10.0 * 128.0);
        assert!(approx(cal.to_volts(Channel::Ch1, VoltageRange::V0_5, 255), expected_small));
    }

    #[test]
    fn to_volts_slice_clears_and_fills_output() {
        let cal = Calibration::default();
        let mut out = vec![9.0; 5];
        cal.to_volts_slice(Channel::Ch2, VoltageRange::V1, &[128, 0], &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], cal.to_volts(Channel::Ch2, VoltageRange::V1, 0));
    }
}
