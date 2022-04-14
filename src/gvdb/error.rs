use safe_transmute::{Error, GuardError};
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum GvdbError {
    UTF8(FromUtf8Error),
    IO(std::io::Error),
    Transmute,
    DataOffset,
    DataAlignment,
    InvalidData,
    DataError(String),
    TooMuchData,
    KeyError,
}

impl From<FromUtf8Error> for GvdbError {
    fn from(err: FromUtf8Error) -> Self {
        Self::UTF8(err)
    }
}

impl From<std::io::Error> for GvdbError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
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
            Error::Guard(gerr) => match gerr {
                GuardError {
                    required,
                    actual,
                    reason: _,
                } => {
                    if actual > required {
                        Self::DataError(format!(
                            "Found {} unexpected trailing bytes at the end while reading data",
                            actual - required
                        ))
                    } else {
                        Self::DataError(format!("Missing {} bytes to read data", actual - required))
                    }
                }
            },
            Error::Unaligned(_) => Self::DataError("Unaligned data read".to_string()),
            _ => Self::InvalidData,
        }
    }
}

pub type GvdbResult<T> = std::result::Result<T, GvdbError>;

#[derive(Debug)]
pub enum GvdbBuilderError {
    WrongParentPrefix,
    EmptyKey,
    InvalidRootChunk,
    IO(std::io::Error),
}

impl From<std::io::Error> for GvdbBuilderError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

pub type GvdbBuilderResult<T> = std::result::Result<T, GvdbBuilderError>;
