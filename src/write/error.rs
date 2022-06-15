use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

/// Error type for GvdbFileWriter
pub enum GvdbWriterError {
    /// Generic I/O error. Path contains an optional filename if applicable
    Io(std::io::Error, Option<PathBuf>),

    /// An internal inconsistency was found
    Consistency(String),

    /// An error occured when serializing variant data with zvariant
    ZVariant(zvariant::Error),
}

impl Error for GvdbWriterError {}

impl From<std::io::Error> for GvdbWriterError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl From<zvariant::Error> for GvdbWriterError {
    fn from(err: zvariant::Error) -> Self {
        Self::ZVariant(err)
    }
}

impl Display for GvdbWriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GvdbWriterError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GvdbWriterError::Consistency(context) => {
                write!(f, "Internal inconsistency: {}", context)
            }
            GvdbWriterError::ZVariant(err) => {
                write!(f, "Error writing ZVariant data: {}", err)
            }
        }
    }
}

impl Debug for GvdbWriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// The Result type for [`GvdbWriterError`]
pub type GvdbBuilderResult<T> = Result<T, GvdbWriterError>;
