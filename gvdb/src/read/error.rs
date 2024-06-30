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

    /// An error occured when deserializing variant data with zvariant
    ZVariant(zvariant::Error),

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
    pub(crate) fn from_io_with_filename(filename: &Path) -> impl FnOnce(std::io::Error) -> Error {
        let path = filename.to_path_buf();
        move |err| Error::Io(err, Some(path))
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
        Self::ZVariant(err)
    }
}

impl From<TryFromIntError> for Error {
    fn from(_err: TryFromIntError) -> Self {
        Self::DataOffset
    }
}

impl<S, T> From<safe_transmute::Error<'_, S, T>> for Error {
    fn from(err: safe_transmute::Error<S, T>) -> Self {
        let name = std::any::type_name::<T>();

        match err {
            safe_transmute::Error::Guard(guard_err) => {
                if guard_err.actual > guard_err.required {
                    Self::Data(format!(
                        "Found {} unexpected trailing bytes at the end while reading {}",
                        guard_err.actual - guard_err.required,
                        name
                    ))
                } else {
                    Self::Data(format!(
                        "Missing {} bytes to read {}",
                        guard_err.required - guard_err.actual,
                        name
                    ))
                }
            }
            safe_transmute::Error::Unaligned(..) => {
                Self::Data(format!("Unaligned data read for {}", name))
            }
            _ => Self::Data(format!("Error transmuting data as {}", name)),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Utf8(err) => write!(f, "Error converting string to UTF-8: {}", err),
            Error::Io(err, path) => {
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
            Error::ZVariant(err) => write!(f, "Error parsing ZVariant data: {}", err),
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
                    "A data inconsistency error occured while reading gvdb file: {}",
                    msg
                )
            }
            Error::KeyNotFound(key) => {
                write!(f, "The item with the key '{}' does not exist", key)
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
    use safe_transmute::{transmute_one_pedantic, transmute_one_to_bytes, transmute_vec};
    use std::num::TryFromIntError;

    #[test]
    fn from() {
        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = Error::Io(io_res.unwrap_err(), None);
        assert!(format!("{}", err).contains("I/O"));

        let utf8_err = String::from_utf8([0xC3, 0x28].to_vec()).unwrap_err();
        let err = Error::from(utf8_err);
        assert!(format!("{}", err).contains("UTF-8"));

        let res: Result<u16, TryFromIntError> = u32::MAX.try_into();
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::DataOffset);
        assert!(format!("{}", err).contains("data offset"));

        let err = Error::Data("my data error".to_string());
        assert!(format!("{}", err).contains("my data error"));

        let err = Error::KeyNotFound("test".to_string());
        assert!(format!("{}", err).contains("test"));

        let err = Error::from(zvariant::Error::Message("test".to_string()));
        assert!(format!("{}", err).contains("test"));

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = transmute_one_to_bytes(&to_transmute).to_vec();
        bytes.extend_from_slice(b"fail");
        let res = transmute_one_pedantic::<Header>(&bytes);
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("unexpected trailing bytes"));

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = transmute_one_to_bytes(&to_transmute).to_vec();
        bytes.remove(bytes.len() - 1);
        let res = transmute_one_pedantic::<Header>(&bytes);
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("Missing 1 bytes"));

        let to_transmute = Header::new(false, 0, Pointer::NULL);
        let mut bytes = b"unalign".to_vec();
        bytes.extend_from_slice(transmute_one_to_bytes(&to_transmute));
        let res = transmute_one_pedantic::<Header>(&bytes[7..]);
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("Unaligned"));

        let bytes = vec![0u8; 5];
        let res = transmute_vec::<u8, Header>(bytes);
        let err = Error::from(res.unwrap_err());
        assert_matches!(err, Error::Data(_));
        assert!(format!("{}", err).contains("transmuting data as gvdb::read::header::Header"));
    }
}
