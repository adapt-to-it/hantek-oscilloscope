/// Errori del driver.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("errore USB: {0}")]
    Usb(String),
    #[error("timeout USB")]
    Timeout,
    #[error("dispositivo Hantek 6022BE non trovato")]
    NotFound,
    #[error("file Intel HEX non valido alla riga {line}: {reason}")]
    InvalidHex { line: usize, reason: String },
    #[error("argomento non valido: {0}")]
    InvalidArgument(String),
    #[error("risposta inattesa dal dispositivo: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, Error>;
