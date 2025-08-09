use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

/// An error that can occur during GVDB file reading
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// Error converting a string to UTF-8
    Utf8(Utf8Error),

    /// Generic I/O error. Path contains an optional filename if applicable
    Io(std::io::Error, Option<PathBuf>),

    /// Tried to access an invalid data offset
    DataOffset,

    /// Tried to read unaligned data
    DataAlignment,

    /// Read invalid data with context information in the provided string
    Data(String),

    /// The item with the specified key does not exist in the hash table
    KeyNotFound(String),
}

impl Error {
    pub(crate) fn from_io_with_filename(filename: &Path) -> impl FnOnce(std::io::Error) -> Error + use<> {
        let path = filename.to_path_buf();
        move |err| Error::Io(err, Some(path))
    }
}

impl<Src, Dst: ?Sized> From<zerocopy::CastError<Src, Dst>> for Error {
    fn from(value: zerocopy::CastError<Src, Dst>) -> Self {
        match value {
            zerocopy::ConvertError::Alignment(_) => Self::DataAlignment,
            zerocopy::ConvertError::Size(_) => Self::DataOffset,
            zerocopy::ConvertError::Validity(_infallible) => unreachable!(),
        }
    }
}

impl std::error::Error for Error {}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err.utf8_error())
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<zvariant::Error> for Error {
    fn from(err: zvariant::Error) -> Self {
        Self::Data(format!("Error deserializing value as gvariant: {err}"))
    }
}

impl From<TryFromIntError> for Error {
    fn from(_err: TryFromIntError) -> Self {
        Self::DataOffset
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Utf8(err) => write!(f, "Error converting string to UTF-8: {err}"),
            Error::Io(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "I/O error while reading file '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "I/O error: {err}")
                }
            }
            Error::DataOffset => {
                write!(f, "Tried to access an invalid data offset. Most likely reason is a corrupted GVDB file")
            }
            Error::DataAlignment => {
                write!(
                    f,
                    "Tried to read unaligned data. Most likely reason is a corrupted GVDB file"
                )
            }
            Error::Data(msg) => {
                write!(
                    f,
                    "A data inconsistency error occured while reading gvdb file: {msg}"
                )
            }
            Error::KeyNotFound(key) => {
                write!(f, "The item with the key '{key}' does not exist")
            }
        }
    }
}

/// The Result type for [`Error`]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use crate::read::{Error, Header, Pointer};
    use matches::assert_matches;
    use std::num::TryFromIntError;
    use zerocopy::{CastError, FromBytes, IntoBytes};

    #[test]
    fn from() {
        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = Error::Io(io_res.unwrap_err(), None);
        assert!(format!("{err}").contains("I/O"));

        let utf8_err = String::from_utf8([0xC3, 0x28].to_vec()).unwrap_err();
        let err = Error::from(utf8_err);
        assert!(format!("{err}").contains("UTF-8"));

        let res: Result<u16, TryFromIntError> = u32::MAX.try_into();
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::DataOffset);
        assert!(format!("{err}").contains("data offset"));

        let err = Error::Data("my data error".to_string());
        assert!(format!("{err}").contains("my data error"));

        let err = Error::KeyNotFound("test".to_string());
        assert!(format!("{err}").contains("test"));

        let err = Error::from(zvariant::Error::Message("test".to_string()));
        assert!(format!("{err}").contains("test"));

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = to_transmute.as_bytes().to_vec();
        bytes.extend_from_slice(b"fail");
        let res = Header::ref_from_bytes(&bytes);
        assert_matches!(res, Err(CastError::Size(_))); // unexpected trailing bytes

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = to_transmute.as_bytes().to_vec();
        bytes.remove(bytes.len() - 1);
        let res = Header::ref_from_bytes(&bytes);
        assert_matches!(res, Err(CastError::Size(_))); //Missing 1 byte

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = b"unalign".to_vec();
        bytes.extend_from_slice(to_transmute.as_bytes());
        let res = Header::ref_from_bytes(&bytes[7..]);
        assert_matches!(res, Err(CastError::Alignment(_))); // Unaligned

        let bytes = vec![0u8; 5];
        let res = <[Header]>::ref_from_bytes(&bytes);
        assert_matches!(res, Err(CastError::Size(_))); // Invalid size
    }
}
