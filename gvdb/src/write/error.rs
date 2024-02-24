use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

/// Error type for [`FileWriter`][crate::write::FileWriter]
#[non_exhaustive]
pub enum Error {
    /// Generic I/O error. Path contains an optional filename if applicable
    Io(std::io::Error, Option<PathBuf>),

    /// An internal inconsistency was found
    Consistency(String),

    /// An error occured when serializing variant data with zvariant
    ZVariant(zvariant::Error),
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl From<zvariant::Error> for Error {
    fn from(err: zvariant::Error) -> Self {
        Self::ZVariant(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            Error::Consistency(context) => {
                write!(f, "Internal inconsistency: {}", context)
            }
            Error::ZVariant(err) => {
                write!(f, "Error writing ZVariant data: {}", err)
            }
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// The Result type for [`Error`]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::Error;
    use matches::assert_matches;
    use std::path::PathBuf;

    #[test]
    fn from() {
        let err = Error::from(zvariant::Error::Message("Test".to_string()));
        assert_matches!(err, Error::ZVariant(_));
        assert!(format!("{}", err).contains("ZVariant"));

        let err = Error::Io(
            std::io::Error::from(std::io::ErrorKind::NotFound),
            Some(PathBuf::from("test_path")),
        );
        assert_matches!(err, Error::Io(..));
        assert!(format!("{}", err).contains("test_path"));
    }
}
