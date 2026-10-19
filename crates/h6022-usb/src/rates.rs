//! Tabelle delle frequenze di campionamento e delle portate verticali.

use crate::types::{ChannelMode, SampleRate, VoltageRange};

/// Portata USB che il firmware sostiene senza interruzioni.
pub const USB_CONTINUOUS_BYTES_PER_SECOND: u64 = 45_000_000;

/// Id firmware in ordine di frequenza crescente.
const ALL: [SampleRate; 25] = [
    SampleRate(102),
    SampleRate(103),
    SampleRate(104),
    SampleRate(105),
    SampleRate(106),
    SampleRate(110),
    SampleRate(113),
    SampleRate(120),
    SampleRate(140),
    SampleRate(150),
    SampleRate(164),
    SampleRate(1),
    SampleRate(2),
    SampleRate(3),
    SampleRate(4),
    SampleRate(5),
    SampleRate(6),
    SampleRate(8),
    SampleRate(10),
    SampleRate(12),
    SampleRate(15),
    SampleRate(16),
    SampleRate(24),
    SampleRate(30),
    SampleRate(48),
];

impl SampleRate {
    /// Tutte le frequenze supportate dal firmware DSO6022BE, dalla più lenta alla più veloce.
    pub fn all() -> &'static [SampleRate] {
        &ALL
    }

    /// Campioni non validi all'inizio di ogni segmento.
    ///
    /// Fino a 24 MS/s il GPIF genera il clock dell'ADC e lo ferma con l'acquisizione:
    /// alla ripartenza la pipeline dell'ADC restituisce 5 valori vecchi.
    /// A 30 e 48 MS/s l'ADC usa IFCLK, che non si ferma.
    pub fn stale_samples_after_restart(self) -> usize {
        match self.0 {
            30 | 48 => 0,
            _ => 5,
        }
    }

    /// `None` se il firmware non conosce l'id.
    pub fn from_id(id: u8) -> Option<SampleRate> {
        ALL.iter().copied().find(|r| r.0 == id)
    }

    /// Frequenza in Hz. 0 se l'id non è noto al firmware.
    pub fn hz(self) -> u32 {
        match self.0 {
            102 => 20_000,
            103 => 32_000,
            104 => 40_000,
            105 => 50_000,
            106 => 64_000,
            110 => 100_000,
            113 => 128_000,
            120 => 200_000,
            140 => 400_000,
            150 => 500_000,
            164 => 640_000,
            1 => 1_000_000,
            2 => 2_000_000,
            3 => 3_000_000,
            4 => 4_000_000,
            5 => 5_000_000,
            6 => 6_000_000,
            8 => 8_000_000,
            10 => 10_000_000,
            12 => 12_000_000,
            15 => 15_000_000,
            16 => 16_000_000,
            24 => 24_000_000,
            30 => 30_000_000,
            48 => 48_000_000,
            _ => 0,
        }
    }

    /// Frequenza supportata più vicina a `hz`.
    pub fn nearest(hz: f64) -> SampleRate {
        let distance = |r: &SampleRate| (f64::from(r.hz()) - hz).abs();
        // `min_by` restituisce il primo minimo: a parità vince la frequenza più bassa.
        ALL.iter()
            .copied()
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
            .unwrap_or(ALL[0])
    }

    /// Byte al secondo che il dispositivo invia con questa frequenza e questi canali.
    pub fn bytes_per_second(self, mode: ChannelMode) -> u64 {
        u64::from(self.hz()) * mode as u64
    }

    /// Vero se il flusso supera la portata USB continua: il firmware lo divide in segmenti
    /// continui di poche migliaia di campioni, separati da interruzioni.
    pub fn is_segmented(self, mode: ChannelMode) -> bool {
        self.bytes_per_second(mode) > USB_CONTINUOUS_BYTES_PER_SECOND
    }

    /// Etichetta leggibile, per esempio "48 MS/s" o "640 kS/s".
    pub fn label(self) -> String {
        let hz = self.hz();
        if hz >= 1_000_000 {
            format!("{} MS/s", hz / 1_000_000)
        } else if hz >= 1_000 {
            format!("{} kS/s", hz / 1_000)
        } else {
            format!("{} S/s", hz)
        }
    }
}

impl VoltageRange {
    pub fn all() -> [VoltageRange; 4] {
        [VoltageRange::V5, VoltageRange::V2_5, VoltageRange::V1, VoltageRange::V0_5]
    }

    /// Codice inviato al firmware (1, 2, 5, 10).
    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<VoltageRange> {
        VoltageRange::all().into_iter().find(|r| r.code() == code)
    }

    /// Semiampiezza della portata in volt: 5.0, 2.5, 1.0, 0.5.
    pub fn half_span_volts(self) -> f32 {
        match self {
            VoltageRange::V5 => 5.0,
            VoltageRange::V2_5 => 2.5,
            VoltageRange::V1 => 1.0,
            VoltageRange::V0_5 => 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_is_strictly_ascending_and_complete() {
        let all = SampleRate::all();
        assert_eq!(all.len(), 25);
        assert!(all.windows(2).all(|w| w[0].hz() < w[1].hz()));
        assert_eq!(all.first().unwrap().hz(), 20_000);
        assert_eq!(all.last().unwrap().hz(), 48_000_000);
    }

    #[test]
    fn from_id_knows_only_firmware_ids() {
        assert_eq!(SampleRate::from_id(102), Some(SampleRate(102)));
        assert_eq!(SampleRate::from_id(48), Some(SampleRate(48)));
        assert_eq!(SampleRate::from_id(0), None);
        assert_eq!(SampleRate::from_id(200), None);
    }

    #[test]
    fn label_uses_mega_kilo_and_units() {
        assert_eq!(SampleRate(48).label(), "48 MS/s");
        assert_eq!(SampleRate(164).label(), "640 kS/s");
        assert_eq!(SampleRate(102).label(), "20 kS/s");
    }

    #[test]
    fn nearest_picks_closest_and_lower_on_tie() {
        assert_eq!(SampleRate::nearest(0.0), SampleRate(102));
        assert_eq!(SampleRate::nearest(1e9), SampleRate(48));
        // Punto medio tra 20 kHz e 32 kHz: vince la più bassa.
        assert_eq!(SampleRate::nearest(26_000.0), SampleRate(102));
        assert_eq!(SampleRate::nearest(1_000_000.0), SampleRate(1));
    }

    #[test]
    fn bytes_per_second_scales_with_channels() {
        assert_eq!(SampleRate(48).bytes_per_second(ChannelMode::Single), 48_000_000);
        assert_eq!(SampleRate(48).bytes_per_second(ChannelMode::Dual), 96_000_000);
    }

    #[test]
    fn segmented_above_45_mb_per_second() {
        assert!(!SampleRate(16).is_segmented(ChannelMode::Dual));
        assert!(!SampleRate(30).is_segmented(ChannelMode::Single));
        assert!(SampleRate(24).is_segmented(ChannelMode::Dual));
        assert!(SampleRate(48).is_segmented(ChannelMode::Single));
    }

    #[test]
    fn voltage_range_codes_round_trip() {
        for range in VoltageRange::all() {
            assert_eq!(VoltageRange::from_code(range.code()), Some(range));
        }
        assert_eq!(VoltageRange::from_code(3), None);
        assert_eq!(VoltageRange::V1.code(), 5);
        assert_eq!(VoltageRange::V0_5.half_span_volts(), 0.5);
        assert_eq!(VoltageRange::V5.half_span_volts(), 5.0);
    }
}
