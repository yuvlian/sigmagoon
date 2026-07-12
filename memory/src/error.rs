use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeltaruneError {
    #[error("Process '{0}' not found")]
    ProcessNotFound(String),

    #[error("Failed to open process (PID: {0})")]
    OpenProcessError(u32),

    #[error("Failed to read memory at address 0x{address:X}")]
    ReadError {
        address: usize,
        #[source]
        source: Option<std::io::Error>,
    },

    #[error("Failed to write memory at address 0x{address:X}")]
    WriteError {
        address: usize,
        #[source]
        source: Option<std::io::Error>,
    },

    #[error("Module '{0}' not found in process")]
    ModuleNotFound(String),

    #[error("Failed to create toolhelp snapshot")]
    SnapshotError,

    #[error("Invalid handle")]
    InvalidHandle,
}

pub type Result<T> = std::result::Result<T, DeltaruneError>;
