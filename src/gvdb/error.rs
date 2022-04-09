use crate::gvdb::util::TryFromByteSliceError;
use deku::prelude::*;
use std::array::TryFromSliceError;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum GvdbError {
    UTF8(FromUtf8Error),
    IO(std::io::Error),
    Deku(DekuError),
    TryFromSlice(TryFromSliceError),
    DataOffset,
    DataAlignment,
    InvalidData,
    DataError(String),
    TooMuchData,
}

impl From<DekuError> for GvdbError {
    fn from(err: DekuError) -> Self {
        Self::Deku(err)
    }
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

impl From<TryFromSliceError> for GvdbError {
    fn from(_err: TryFromSliceError) -> Self {
        Self::DataOffset
    }
}

impl From<TryFromByteSliceError> for GvdbError {
    fn from(_err: TryFromByteSliceError) -> Self {
        Self::DataOffset
    }
}

pub type GvdbResult<T> = std::result::Result<T, GvdbError>;
