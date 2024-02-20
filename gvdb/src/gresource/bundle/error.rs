use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

/// Error type for creating a GResource XML file
#[non_exhaustive]
pub enum BuilderError {
    /// An internal error occurred during creation of the GVDB file
    Gvdb(crate::write::Error),

    /// I/O error
    Io(std::io::Error, Option<PathBuf>),

    /// This error can occur when using xml-stripblanks and the provided XML file is invalid
    Xml(quick_xml::Error, Option<PathBuf>),

    /// A file needs to be interpreted as UTF-8 (for stripping whitespace etc.) but it is invalid
    Utf8(std::str::Utf8Error, Option<PathBuf>),

    /// This error can occur when using json-stripblanks and the provided JSON file is invalid
    Json(serde_json::Error, Option<PathBuf>),

    /// Error when canonicalizing a path from an absolute to a relative path
    StripPrefix(std::path::StripPrefixError, PathBuf),

    /// This feature is not implemented in gvdb-rs
    Unimplemented(String),
}

impl BuilderError {
    pub(crate) fn from_io_with_filename<P>(
        filename: Option<P>,
    ) -> impl FnOnce(std::io::Error) -> BuilderError
    where
        P: Into<PathBuf>,
    {
        let path = filename.map(|p| p.into());
        move |err| BuilderError::Io(err, path)
    }
}

impl std::error::Error for BuilderError {}

impl From<crate::write::Error> for BuilderError {
    fn from(err: crate::write::Error) -> Self {
        Self::Gvdb(err)
    }
}

impl Display for BuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuilderError::Xml(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "Error processing XML data for file '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "Error processing XML data: {}", err)
                }
            }
            BuilderError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            BuilderError::Json(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "Error parsing JSON from file: '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "Error reading/writing JSON data: {}", err)
                }
            }
            BuilderError::Utf8(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "Error converting file '{}' to UTF-8: {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "Error converting data to UTF-8: {}", err)
                }
            }
            BuilderError::Unimplemented(err) => {
                write!(f, "{}", err)
            }
            BuilderError::Gvdb(err) => {
                write!(f, "Error while creating GVDB file: {:?}", err)
            }
            BuilderError::StripPrefix(err, path) => {
                write!(
                    f,
                    "Error when canonicalizing path '{:?}' from an absolute to a relative path: {}",
                    path, err
                )
            }
        }
    }
}

impl Debug for BuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// Result type for [`BuilderError`]
pub type BuilderResult<T> = std::result::Result<T, BuilderError>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from() {
        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = BuilderError::Io(io_res.unwrap_err(), None);
        assert!(format!("{}", err).contains("I/O"));

        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = BuilderError::from_io_with_filename(Some("test"))(io_res.unwrap_err());
        assert!(format!("{}", err).contains("test"));

        let writer_error = crate::write::Error::Consistency("test".to_string());
        let err = BuilderError::from(writer_error);
        assert!(format!("{}", err).contains("test"));

        let err = BuilderError::Xml(
            quick_xml::Error::TextNotFound,
            Some(PathBuf::from("test_file")),
        );
        assert!(format!("{}", err).contains("test_file"));
        let err = BuilderError::Xml(quick_xml::Error::TextNotFound, None);
        assert!(format!("{}", err).contains("XML"));
    }
}
