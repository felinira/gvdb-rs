use crate::write::GvdbWriterError;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

/// Error when parsing a GResource XML file
pub enum GResourceXMLError {
    /// An error occured during parsing of the XML file
    Serde(serde_xml_rs::Error, Option<std::path::PathBuf>),

    /// Generic I/O error occurred when handling XML file
    Io(std::io::Error, Option<std::path::PathBuf>),
}

impl GResourceXMLError {
    pub(crate) fn from_io_with_filename(
        filename: &Path,
    ) -> impl FnOnce(std::io::Error) -> GResourceXMLError {
        let path = filename.to_path_buf();
        move |err| GResourceXMLError::Io(err, Some(path))
    }

    pub(crate) fn from_serde_with_filename(
        filename: Option<&Path>,
    ) -> impl FnOnce(serde_xml_rs::Error) -> GResourceXMLError {
        let path = filename.map(|f| f.to_path_buf());
        move |err| GResourceXMLError::Serde(err, path)
    }
}

impl std::error::Error for GResourceXMLError {}

impl Display for GResourceXMLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GResourceXMLError::Serde(err, path) => {
                if let Some(path) = path {
                    write!(f, "Error parsing XML file '{}': {}", path.display(), err)
                } else {
                    write!(f, "Error parsing XML file: {}", err)
                }
            }
            GResourceXMLError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
        }
    }
}

impl Debug for GResourceXMLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// Result type for GResourceXMLError
pub type GResourceXMLResult<T> = Result<T, GResourceXMLError>;

/// Error type for creating a GResource XML file
pub enum GResourceBuilderError {
    /// An internal error occurred during creation of the GVDB file
    Gvdb(GvdbWriterError),

    /// I/O error
    Io(std::io::Error, Option<std::path::PathBuf>),

    /// This error can occur when using xml-stripblanks and the provided XML file is invalid
    XmlRead(xml::reader::Error, Option<std::path::PathBuf>),

    /// This error can occur when using xml-stripblanks and the provided XML file is invalid
    XmlWrite(xml::writer::Error, Option<std::path::PathBuf>),

    /// A file needs to be interpreted as UTF-8 (for stripping whitespace etc.) but it is invalid
    Utf8(std::string::FromUtf8Error, Option<std::path::PathBuf>),

    /// This error can occur when using json-stripblanks and the provided JSON file is invalid
    Json(json::Error, Option<std::path::PathBuf>),

    /// This feature is not implemented in gvdb-rs
    Unimplemented(String),

    /// A generic error with a text description
    Generic(String),
}

impl GResourceBuilderError {
    pub(crate) fn from_io_with_filename<P>(
        filename: Option<P>,
    ) -> impl FnOnce(std::io::Error) -> GResourceBuilderError
    where
        P: Into<PathBuf>,
    {
        let path = filename.map(|p| p.into());
        move |err| GResourceBuilderError::Io(err, path)
    }
}

impl std::error::Error for GResourceBuilderError {}

impl From<GvdbWriterError> for GResourceBuilderError {
    fn from(err: GvdbWriterError) -> Self {
        Self::Gvdb(err)
    }
}

impl Display for GResourceBuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GResourceBuilderError::XmlRead(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "Error reading XML data for file '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "Error reading XML data: {}", err)
                }
            }
            GResourceBuilderError::XmlWrite(err, path) => {
                if let Some(path) = path {
                    write!(
                        f,
                        "Error writing XML data for file '{}': {}",
                        path.display(),
                        err
                    )
                } else {
                    write!(f, "Error writing XML data: {}", err)
                }
            }
            GResourceBuilderError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GResourceBuilderError::Json(err, path) => {
                if let Some(path) = path {
                    write!(f, "Error parsing JSON from file: {}", path.display())
                } else {
                    write!(f, "Error reading/writing JSON data: {}", err)
                }
            }
            GResourceBuilderError::Utf8(err, path) => {
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
            GResourceBuilderError::Unimplemented(err) => {
                write!(f, "{}", err)
            }
            GResourceBuilderError::Gvdb(err) => {
                write!(f, "Error while creating GVDB file: {:?}", err)
            }
            GResourceBuilderError::Generic(err) => {
                write!(f, "Error while creating GResource file: {}", err)
            }
        }
    }
}

impl Debug for GResourceBuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// Result type for [`GResourceBuilderError`]
pub type GResourceBuilderResult<T> = Result<T, GResourceBuilderError>;

#[cfg(test)]
mod test {
    use crate::gresource::{GResourceBuilderError, GResourceXMLError};
    use crate::write::GvdbWriterError;

    #[test]
    fn from() {
        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = GResourceXMLError::Io(io_res.unwrap_err(), None);
        assert!(format!("{}", err).contains("I/O"));

        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = GResourceBuilderError::Io(io_res.unwrap_err(), None);
        assert!(format!("{}", err).contains("I/O"));

        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = GResourceBuilderError::from_io_with_filename(Some("test"))(io_res.unwrap_err());
        assert!(format!("{}", err).contains("test"));

        let writer_error = GvdbWriterError::Consistency("test".to_string());
        let err = GResourceBuilderError::from(writer_error);
        assert!(format!("{}", err).contains("test"));
    }
}
