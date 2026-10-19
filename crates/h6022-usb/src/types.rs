//! Costanti del protocollo e tipi condivisi. Nessuna logica: i metodi stanno in `rates.rs`.

/// VID prima del caricamento del firmware.
pub const VID_NO_FIRMWARE: u16 = 0x04b4;
/// VID dopo il caricamento del firmware: il firmware si rienumera con questo VID.
pub const VID_FIRMWARE: u16 = 0x04b5;
pub const PID_6022BE: u16 = 0x6022;

/// Registro CPUCS del FX2: 1 ferma l'8051, 0 lo avvia.
pub const CPUCS_ADDR: u16 = 0xe600;
/// Endpoint bulk IN dei campioni (interfaccia 0, alt 0).
pub const BULK_EP_IN: u8 = 0x86;
/// Inizio della calibrazione nella EEPROM.
pub const CALIBRATION_EEPROM_OFFSET: u16 = 0x08;
/// 32 byte di fabbrica + 48 byte estesi.
pub const CALIBRATION_EEPROM_LEN: u16 = 80;

/// Richieste vendor (bRequest). Tutte con value = 0 e index = 0, salvo dove indicato.
pub mod req {
    /// Scrittura/lettura RAM del FX2. value = indirizzo.
    pub const FIRMWARE: u8 = 0xa0;
    /// Lettura (IN) o scrittura (OUT) EEPROM. value = offset.
    pub const EEPROM: u8 = 0xa2;
    pub const CH1_RANGE: u8 = 0xe0;
    pub const CH2_RANGE: u8 = 0xe1;
    pub const SAMPLE_RATE: u8 = 0xe2;
    /// data = [1] avvia, [0] ferma.
    pub const START_STOP: u8 = 0xe3;
    pub const NUM_CHANNELS: u8 = 0xe4;
    /// data = [bit0: CH1 DC, bit4: CH2 DC].
    pub const COUPLING: u8 = 0xe5;
    pub const CAL_FREQ: u8 = 0xe6;
    /// IN, 4 byte: versione (little endian), bit di capacità, riservato. Dal firmware 3.00.
    pub const INFO: u8 = 0xe7;
    /// IN, 2 byte: contatore di FIFO piena (little endian). Non ferma il campionamento.
    pub const OVERFLOWS: u8 = 0xe8;
}

/// Versione del firmware incorporato, come in `bcdDevice`.
pub const FIRMWARE_VERSION: u16 = 0x0300;

/// Capacità dichiarate dal firmware con la richiesta `req::INFO`.
pub mod cap {
    /// Il firmware conta le volte in cui la FIFO si riempie durante il campionamento.
    pub const OVERFLOW_COUNTER: u8 = 0x01;
}

/// Risposta alla richiesta `req::INFO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareInfo {
    pub version: u16,
    /// Bit definiti in `cap`.
    pub capabilities: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Ch1,
    Ch2,
}

/// Numero di canali acquisiti. Il discriminante è il valore inviato al firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelMode {
    Single = 1,
    Dual = 2,
}

/// Portata verticale. Il discriminante è il codice del firmware (guadagno).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoltageRange {
    /// ±5 V
    V5 = 1,
    /// ±2,5 V
    V2_5 = 2,
    /// ±1 V
    V1 = 5,
    /// ±0,5 V
    V0_5 = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Coupling {
    Ac,
    Dc,
}

/// Frequenza di campionamento, identificata dall'id del firmware (comando 0xe2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleRate(pub u8);
