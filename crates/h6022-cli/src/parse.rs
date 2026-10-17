//! Parsing degli argomenti da riga di comando. Funzioni pure, senza accesso al dispositivo.

use h6022_usb::{ChannelMode, SampleRate, VoltageRange};

/// Frequenza in Hz. Accetta un suffisso `k` (chilo) o `M` (mega), per esempio "48M", "500k", "20000".
pub fn parse_frequency(s: &str) -> Result<f64, String> {
    let t = s.trim();
    let (num, mult) = match t.chars().last() {
        Some('M' | 'm') => (&t[..t.len() - 1], 1e6),
        Some('k' | 'K') => (&t[..t.len() - 1], 1e3),
        _ => (t, 1.0),
    };
    let value: f64 = num
        .trim()
        .parse()
        .map_err(|_| format!("frequenza non valida: \"{s}\""))?;
    let hz = value * mult;
    if !hz.is_finite() || hz <= 0.0 {
        return Err(format!("frequenza non valida: \"{s}\""));
    }
    Ok(hz)
}

/// Frequenza supportata più vicina a quella richiesta.
pub fn parse_rate(s: &str) -> Result<SampleRate, String> {
    parse_frequency(s).map(SampleRate::nearest)
}

/// Portata verticale in volt: 5, 2.5, 1, 0.5.
pub fn parse_range(s: &str) -> Result<VoltageRange, String> {
    match s.trim() {
        "5" => Ok(VoltageRange::V5),
        "2.5" => Ok(VoltageRange::V2_5),
        "1" => Ok(VoltageRange::V1),
        "0.5" => Ok(VoltageRange::V0_5),
        _ => Err(format!(
            "portata non supportata: \"{s}\" (valori ammessi: 5, 2.5, 1, 0.5)"
        )),
    }
}

/// Numero di canali: 1 o 2.
pub fn parse_channels(s: &str) -> Result<ChannelMode, String> {
    match s.trim() {
        "1" => Ok(ChannelMode::Single),
        "2" => Ok(ChannelMode::Dual),
        _ => Err(format!("canali non validi: \"{s}\" (valori ammessi: 1, 2)")),
    }
}

/// Numero a 16 bit in decimale o in esadecimale con prefisso `0x`.
pub fn parse_u16(s: &str) -> Result<u16, String> {
    let t = s.trim();
    let parsed = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u16::from_str_radix(hex, 16),
        None => t.parse::<u16>(),
    };
    parsed.map_err(|_| format!("numero non valido: \"{s}\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequenza_con_suffissi() {
        assert_eq!(parse_frequency("48M"), Ok(48e6));
        assert_eq!(parse_frequency("500k"), Ok(500e3));
        assert_eq!(parse_frequency("20000"), Ok(20000.0));
        assert_eq!(parse_frequency(" 1.5M "), Ok(1.5e6));
    }

    #[test]
    fn frequenza_non_valida() {
        for s in ["", "M", "abc", "-1", "0", "nan", "inf", "12x"] {
            assert!(parse_frequency(s).is_err(), "accettata: {s:?}");
        }
    }

    #[test]
    fn portata_ammesse() {
        assert_eq!(parse_range("5"), Ok(VoltageRange::V5));
        assert_eq!(parse_range("2.5"), Ok(VoltageRange::V2_5));
        assert_eq!(parse_range("1"), Ok(VoltageRange::V1));
        assert_eq!(parse_range("0.5"), Ok(VoltageRange::V0_5));
        assert_eq!(parse_range(" 1 "), Ok(VoltageRange::V1));
    }

    #[test]
    fn portata_non_ammesse() {
        for s in ["", "2", "5.0", "10", "0,5", "abc"] {
            assert!(parse_range(s).is_err(), "accettata: {s:?}");
        }
    }

    #[test]
    fn canali_ammessi() {
        assert_eq!(parse_channels("1"), Ok(ChannelMode::Single));
        assert_eq!(parse_channels("2"), Ok(ChannelMode::Dual));
        assert!(parse_channels("3").is_err());
        assert!(parse_channels("").is_err());
    }

    #[test]
    fn numeri_u16() {
        assert_eq!(parse_u16("80"), Ok(80));
        assert_eq!(parse_u16("0x08"), Ok(8));
        assert_eq!(parse_u16("0X10"), Ok(16));
        assert!(parse_u16("70000").is_err());
        assert!(parse_u16("0x10000").is_err());
        assert!(parse_u16("x").is_err());
    }
}
