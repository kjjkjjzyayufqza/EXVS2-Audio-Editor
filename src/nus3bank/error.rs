use std::fmt;

/// NUS3BANK specific error types
#[derive(Debug)]
pub enum Nus3bankError {
    /// Invalid magic number in file header or section
    InvalidMagic { expected: String, found: String },
    /// Section validation failed
    SectionValidation { section: String },
    /// String alignment error
    StringAlignment { size: usize, padding: usize },
    /// File reconstruction failed
    Reconstruction { reason: String },
    /// Track not found by hex ID
    TrackNotFound { hex_id: String },
    /// Invalid hex ID format
    InvalidHexId { hex_id: String },
    /// IO error
    Io(std::io::Error),
    /// UTF-8 conversion error
    Utf8(std::string::FromUtf8Error),

    /// Invalid file format
    InvalidFormat { reason: String },
}

impl fmt::Display for Nus3bankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nus3bankError::InvalidMagic { expected, found } => {
                write!(f, "Invalid magic number: expected {}, found {}", expected, found)
            }
            Nus3bankError::SectionValidation { section } => {
                write!(f, "Section validation failed: {}", section)
            }
            Nus3bankError::StringAlignment { size, padding } => {
                write!(f, "String alignment error: size {}, padding {}", size, padding)
            }
            Nus3bankError::Reconstruction { reason } => {
                write!(f, "File reconstruction failed: {}", reason)
            }
            Nus3bankError::TrackNotFound { hex_id } => {
                write!(f, "Track not found: {}", hex_id)
            }
            Nus3bankError::InvalidHexId { hex_id } => {
                write!(f, "Invalid hex ID format: {}", hex_id)
            }
            Nus3bankError::Io(err) => write!(f, "IO error: {}", err),
            Nus3bankError::Utf8(err) => write!(f, "UTF-8 conversion error: {}", err),

            Nus3bankError::InvalidFormat { reason } => {
                write!(f, "Invalid file format: {}", reason)
            }
        }
    }
}

impl std::error::Error for Nus3bankError {}

impl From<std::io::Error> for Nus3bankError {
    fn from(err: std::io::Error) -> Self {
        Nus3bankError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for Nus3bankError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Nus3bankError::Utf8(err)
    }
}
