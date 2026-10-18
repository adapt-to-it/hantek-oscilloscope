//! Scelta del decoder da un'unica richiesta, condivisa da server e GUI.

use h6022_dsp::Waveform;

use crate::can::CanConfig;
use crate::uart::UartConfig;
use crate::{Annotation, can, digitize, i2c, ir, manchester, onewire, pwm, uart};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protocol {
    Uart(UartConfig),
    /// SCL sul canale indicato dalla richiesta, SDA sull'altro.
    I2c,
    Can(CanConfig),
    OneWire,
    /// Periodo di bit (`None`: stima automatica) e convenzione IEEE.
    Manchester { bit_period_s: Option<f64>, ieee: bool },
    Pwm,
    Servo,
    IrNec,
    IrRc5,
}

impl Protocol {
    /// Nome breve usato da MCP e GUI.
    pub fn name(&self) -> &'static str {
        match self {
            Protocol::Uart(_) => "uart",
            Protocol::I2c => "i2c",
            Protocol::Can(_) => "can",
            Protocol::OneWire => "onewire",
            Protocol::Manchester { .. } => "manchester",
            Protocol::Pwm => "pwm",
            Protocol::Servo => "servo",
            Protocol::IrNec => "ir_nec",
            Protocol::IrRc5 => "ir_rc5",
        }
    }

    pub fn needs_two_channels(&self) -> bool {
        matches!(self, Protocol::I2c)
    }
}

/// Decodifica `primary` (per I2C è SCL) e, se serve, `secondary` (SDA).
/// `threshold` in volt: `None` per la soglia automatica di `digitize`.
pub fn decode(
    protocol: &Protocol,
    primary: &Waveform,
    secondary: Option<&Waveform>,
    threshold: Option<f32>,
) -> Result<Vec<Annotation>, String> {
    let trace = digitize(primary, threshold);
    Ok(match protocol {
        Protocol::Uart(config) => uart::decode(&trace, config),
        Protocol::I2c => {
            let sda = secondary.ok_or("I2C richiede due canali: SCL e SDA")?;
            i2c::decode(&trace, &digitize(sda, threshold))
        }
        Protocol::Can(config) => can::decode(&trace, config),
        Protocol::OneWire => onewire::decode(&trace),
        Protocol::Manchester { bit_period_s, ieee } => manchester::decode(&trace, *bit_period_s, *ieee),
        Protocol::Pwm => pwm::annotate(&trace),
        Protocol::Servo => pwm::decode_servo(&trace),
        Protocol::IrNec => ir::decode_nec(&trace),
        Protocol::IrRc5 => ir::decode_rc5(&trace),
    })
}

/// Testo riassuntivo: i byte "data" in esadecimale e, se tutti stampabili, in ASCII.
pub fn summary(annotations: &[Annotation]) -> String {
    let bytes: Vec<u8> = annotations
        .iter()
        .filter(|a| a.kind == "data")
        .filter_map(|a| a.value.and_then(|v| u8::try_from(v).ok()))
        .collect();
    let errors = annotations.iter().filter(|a| a.error).count();
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
    let mut text = format!("{} byte, {} errori", bytes.len(), errors);
    if !bytes.is_empty() {
        text.push_str(&format!(": {}", hex.join(" ")));
        if bytes.iter().all(|b| (0x20..0x7f).contains(b) || *b == b'\n' || *b == b'\r') {
            text.push_str(&format!(" \"{}\"", String::from_utf8_lossy(&bytes).escape_default()));
        }
    }
    text
}
