use thiserror::Error;

#[derive(Error, Debug)]
pub enum RsSpyError {
    #[error("process monitoring error: {0}")]
    Process(#[from] procfs::ProcError),

    #[error("filesystem watching error: {0}")]
    Filesystem(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("dbus error: {0}")]
    DBus(#[from] dbus::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("scanner error: {0}")]
    Scanner(String),

    #[error("unknown error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RsSpyError>;

impl From<String> for RsSpyError {
    fn from(msg: String) -> Self {
        RsSpyError::Other(msg)
    }
}

impl From<&str> for RsSpyError {
    fn from(msg: &str) -> Self {
        RsSpyError::Other(msg.to_string())
    }
}
