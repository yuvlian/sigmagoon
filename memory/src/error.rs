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

    #[error("Offset is unimplemented for chapter {0}")]
    UnimplementedOffset(usize),

    #[error("Failed to get chapter: window title not found or empty")]
    ChapterWindowNotFound,

    #[error("Failed to get chapter: no valid chapter number found in window title")]
    ChapterNumberNotFound,

    #[error("Failed to parse chapter number: '{0}'")]
    ChapterParseError(String),

    #[error("Invalid chapter number {0}: must be between 1 and 7")]
    InvalidChapterNumber(usize),
}

pub type Result<T> = std::result::Result<T, DeltaruneError>;
