use crate::write::error::GvdbWriterError;
use serde_xml_rs::Error;
use std::fmt::{Debug, Display, Formatter};
use std::string::FromUtf8Error;

/// Error when parsing a GResource XML file
pub enum GResourceXMLError {
    /// An error occured during parsing of the XML file
    Serde(serde_xml_rs::Error, Option<std::path::PathBuf>),

    /// Generic I/O error occurred when handling XML file
    Io(std::io::Error, Option<std::path::PathBuf>),
}

impl From<serde_xml_rs::Error> for GResourceXMLError {
    fn from(err: Error) -> Self {
        Self::Serde(err, None)
    }
}

impl From<std::io::Error> for GResourceXMLError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

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
}

impl From<GvdbWriterError> for GResourceBuilderError {
    fn from(err: GvdbWriterError) -> Self {
        Self::Gvdb(err)
    }
}

impl From<xml::reader::Error> for GResourceBuilderError {
    fn from(err: xml::reader::Error) -> Self {
        Self::XmlRead(err, None)
    }
}

impl From<xml::writer::Error> for GResourceBuilderError {
    fn from(err: xml::writer::Error) -> Self {
        Self::XmlWrite(err, None)
    }
}

impl From<std::io::Error> for GResourceBuilderError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl From<json::Error> for GResourceBuilderError {
    fn from(err: json::Error) -> Self {
        Self::Json(err, None)
    }
}

impl From<std::string::FromUtf8Error> for GResourceBuilderError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8(err, None)
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
