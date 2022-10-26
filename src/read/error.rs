use safe_transmute::GuardError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;

/// An error that can occur during GVDB file reading
#[derive(Debug)]
pub enum GvdbReaderError {
    /// Error converting a string to UTF-8
    Utf8(FromUtf8Error),

    /// Generic I/O error. Path contains an optional filename if applicable
    Io(std::io::Error, Option<PathBuf>),

    /// An error occured when deserializing variant data with zvariant
    ZVariant(zvariant::Error),

    /// Tried to access an invalid data offset
    DataOffset,

    /// Tried to read unaligned data
    DataAlignment,

    /// Unexpected data
    InvalidData,

    /// Like InvalidData but with context information in the provided string
    DataError(String),

    /// The item with the specified key does not exist in the hash table
    KeyError(String),
}

impl GvdbReaderError {
    pub(crate) fn from_io_with_filename(
        filename: &Path,
    ) -> impl FnOnce(std::io::Error) -> GvdbReaderError {
        let path = filename.to_path_buf();
        move |err| GvdbReaderError::Io(err, Some(path))
    }
}

impl Error for GvdbReaderError {}

impl From<FromUtf8Error> for GvdbReaderError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<std::io::Error> for GvdbReaderError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl From<zvariant::Error> for GvdbReaderError {
    fn from(err: zvariant::Error) -> Self {
        Self::ZVariant(err)
    }
}

impl From<TryFromIntError> for GvdbReaderError {
    fn from(_err: TryFromIntError) -> Self {
        Self::DataOffset
    }
}

impl<S, T> From<safe_transmute::Error<'_, S, T>> for GvdbReaderError {
    fn from(err: safe_transmute::Error<S, T>) -> Self {
        match err {
            safe_transmute::Error::Guard(GuardError {
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
            safe_transmute::Error::Unaligned(_) => {
                Self::DataError("Unaligned data read".to_string())
            }
            _ => Self::InvalidData,
        }
    }
}

impl Display for GvdbReaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GvdbReaderError::Utf8(err) => write!(f, "Error converting string to UTF-8: {}", err),
            GvdbReaderError::Io(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "I/O error while reading file '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GvdbReaderError::ZVariant(err) => write!(f, "Error parsing ZVariant data: {}", err),
            GvdbReaderError::DataOffset => {
                write!(f, "Tried to access an invalid data offset. Most likely reason is a corrupted GVDB file")
            }
            GvdbReaderError::DataAlignment => {
                write!(
                    f,
                    "Tried to read unaligned data. Most likely reason is a corrupted GVDB file"
                )
            }
            GvdbReaderError::InvalidData => {
                write!(
                    f,
                    "Unexpected data. Most likely reason is a corrupted GVDB file"
                )
            }
            GvdbReaderError::DataError(msg) => {
                write!(
                    f,
                    "A data inconsistency error occured while reading gvdb file: {}",
                    msg
                )
            }
            GvdbReaderError::KeyError(key) => {
                write!(f, "The item with the key '{}' does not exist", key)
            }
        }
    }
}

/// The Result type for [`GvdbReaderError`]
pub type GvdbReaderResult<T> = Result<T, GvdbReaderError>;
