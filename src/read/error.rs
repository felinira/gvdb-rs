use safe_transmute::{Error, GuardError};
use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use std::path::PathBuf;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum GvdbError {
    Utf8(FromUtf8Error),
    Io(std::io::Error, Option<PathBuf>),
    DataOffset,
    DataAlignment,
    InvalidData,
    DataError(String),
    KeyError(String),
}

impl From<FromUtf8Error> for GvdbError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<std::io::Error> for GvdbError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl From<TryFromIntError> for GvdbError {
    fn from(_err: TryFromIntError) -> Self {
        Self::DataOffset
    }
}

impl<S, T> From<safe_transmute::Error<'_, S, T>> for GvdbError {
    fn from(err: Error<S, T>) -> Self {
        match err {
            Error::Guard(GuardError {
                required,
                actual,
                reason: _,
            }) => {
                if actual > required {
                    Self::DataError(format!(
                        "Found {} unexpected trailing bytes at the end while reading data",
                        actual - required
                    ))
                } else {
                    Self::DataError(format!("Missing {} bytes to read data", actual - required))
                }
            }
            Error::Unaligned(_) => Self::DataError("Unaligned data read".to_string()),
            _ => Self::InvalidData,
        }
    }
}

impl Display for GvdbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GvdbError::Utf8(err) => write!(f, "Error converting string to UTF-8: {}", err),
            GvdbError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GvdbError::DataOffset => {
                write!(f, "Tried to access an invalid data offset. Most likely reason is a corrupted GVDB file")
            }
            GvdbError::DataAlignment => {
                write!(
                    f,
                    "Tried to read unaligned data. Most likely reason is a corrupted GVDB file"
                )
            }
            GvdbError::InvalidData => {
                write!(
                    f,
                    "Unexpected data. Most likely reason is a corrupted GVDB file"
                )
            }
            GvdbError::DataError(msg) => {
                write!(
                    f,
                    "A data inconsistency error occured while reading gvdb file: {}",
                    msg
                )
            }
            GvdbError::KeyError(key) => {
                write!(f, "The item with the key '{}' does not exist", key)
            }
        }
    }
}

pub type GvdbResult<T> = std::result::Result<T, GvdbError>;
