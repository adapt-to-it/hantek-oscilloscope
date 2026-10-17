//! Lettura di file Intel HEX.

use crate::{Error, Result};

/// Record dati: indirizzo di partenza e byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexRecord {
    pub address: u16,
    pub data: Vec<u8>,
}

/// Esito di una riga decodificata.
enum Decoded {
    Data(HexRecord),
    Eof,
}

/// Restituisce i record dati in ordine di file. Si ferma al record di fine file.
/// Errore `InvalidHex` per checksum errato, lunghezza incoerente o tipo di record sconosciuto.
pub fn parse(text: &str) -> Result<Vec<HexRecord>> {
    let mut records = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        match decode(line) {
            Ok(Decoded::Data(record)) => records.push(record),
            Ok(Decoded::Eof) => break,
            Err(reason) => {
                return Err(Error::InvalidHex {
                    line: index + 1,
                    reason,
                });
            }
        }
    }
    Ok(records)
}

/// Decodifica una riga `:LLAAAATT...CC` già privata degli spazi finali.
fn decode(line: &str) -> std::result::Result<Decoded, String> {
    let body = line
        .strip_prefix(':')
        .ok_or("manca il carattere ':' iniziale")?;
    // Controllo prima dell'affettamento: solo cifre esadecimali ASCII, in coppie.
    if body.len() % 2 != 0 || !body.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("cifre esadecimali non valide o numero dispari".to_string());
    }
    let bytes: Vec<u8> = (0..body.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&body[i..i + 2], 16))
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| e.to_string())?;

    if bytes.len() < 5 {
        return Err("record troppo corto".to_string());
    }
    let len = usize::from(bytes[0]);
    if bytes.len() != len + 5 {
        return Err("lunghezza dichiarata incoerente con i dati".to_string());
    }
    // La somma di tutti i byte, checksum incluso, deve essere 0 modulo 256.
    if bytes.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)) != 0 {
        return Err("checksum errato".to_string());
    }

    let address = u16::from_be_bytes([bytes[1], bytes[2]]);
    match bytes[3] {
        0x00 => Ok(Decoded::Data(HexRecord {
            address,
            data: bytes[4..4 + len].to_vec(),
        })),
        0x01 => Ok(Decoded::Eof),
        kind => Err(format!("tipo di record {kind:02X} non supportato")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prima riga del firmware di riferimento: 16 byte a 0x0100, checksum valido.
    const DATA_LINE: &str = ":10010000214601360121470136007EFE09D2190140";
    const EOF_LINE: &str = ":00000001FF";

    fn expected_first() -> HexRecord {
        HexRecord {
            address: 0x0100,
            data: vec![
                0x21, 0x46, 0x01, 0x36, 0x01, 0x21, 0x47, 0x01, 0x36, 0x00, 0x7E, 0xFE, 0x09, 0xD2, 0x19, 0x01,
            ],
        }
    }

    #[test]
    fn parses_data_and_stops_at_eof() {
        let text = format!("{DATA_LINE}\n{EOF_LINE}\n:this is not read");
        let records = parse(&text).unwrap();
        assert_eq!(records, vec![expected_first()]);
    }

    #[test]
    fn ignores_blank_lines_and_trailing_spaces() {
        let text = format!("\n{DATA_LINE}   \n   \n\r\n{EOF_LINE}\t \n");
        let records = parse(&text).unwrap();
        assert_eq!(records, vec![expected_first()]);
    }

    #[test]
    fn empty_file_has_no_records() {
        assert_eq!(parse("").unwrap(), Vec::new());
    }

    #[test]
    fn bad_checksum_reports_line_number() {
        let bad = ":10010000214601360121470136007EFE09D2190141";
        let text = format!("{DATA_LINE}\n{bad}\n{EOF_LINE}");
        match parse(&text) {
            Err(Error::InvalidHex { line, .. }) => assert_eq!(line, 2),
            other => panic!("atteso InvalidHex, ottenuto {other:?}"),
        }
    }

    #[test]
    fn missing_colon_is_error_with_line_from_one() {
        match parse("\n\ngarbage") {
            Err(Error::InvalidHex { line, .. }) => assert_eq!(line, 3),
            other => panic!("atteso InvalidHex, ottenuto {other:?}"),
        }
    }

    #[test]
    fn unknown_record_type_is_error() {
        // Tipo 04 (indirizzo lineare esteso), checksum valido: non supportato.
        match parse(":020000040000FA") {
            Err(Error::InvalidHex { line, reason }) => {
                assert_eq!(line, 1);
                assert!(reason.contains("04"));
            }
            other => panic!("atteso InvalidHex, ottenuto {other:?}"),
        }
    }

    #[test]
    fn inconsistent_length_is_error() {
        // Dichiara 0x10 byte ma ne contiene uno solo.
        assert!(matches!(parse(":1001000021"), Err(Error::InvalidHex { line: 1, .. })));
    }
}
