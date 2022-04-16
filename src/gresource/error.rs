use crate::gvdb::write::error::GvdbBuilderError;
use serde_xml_rs::Error;
use std::fmt::{Debug, Display, Formatter};
use std::string::FromUtf8Error;

pub enum GResourceXMLError {
    Serde(serde_xml_rs::Error, Option<std::path::PathBuf>),
    IO(std::io::Error, Option<std::path::PathBuf>),
}

impl From<serde_xml_rs::Error> for GResourceXMLError {
    fn from(err: Error) -> Self {
        Self::Serde(err, None)
    }
}

impl From<std::io::Error> for GResourceXMLError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err, None)
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
            GResourceXMLError::IO(err, path) => {
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

pub type GResourceXMLResult<T> = Result<T, GResourceXMLError>;

pub enum GResourceBuilderError {
    Gvdb(GvdbBuilderError),
    IO(std::io::Error, Option<std::path::PathBuf>),
    XMLRead(xml::reader::Error, Option<std::path::PathBuf>),
    XMLWrite(xml::writer::Error, Option<std::path::PathBuf>),
    UTF8(std::string::FromUtf8Error, Option<std::path::PathBuf>),
    JSON(json::Error, Option<std::path::PathBuf>),
    Unimplemented(String),
}

impl From<GvdbBuilderError> for GResourceBuilderError {
    fn from(err: GvdbBuilderError) -> Self {
        Self::Gvdb(err)
    }
}

impl From<xml::reader::Error> for GResourceBuilderError {
    fn from(err: xml::reader::Error) -> Self {
        Self::XMLRead(err, None)
    }
}

impl From<xml::writer::Error> for GResourceBuilderError {
    fn from(err: xml::writer::Error) -> Self {
        Self::XMLWrite(err, None)
    }
}

impl From<std::io::Error> for GResourceBuilderError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err, None)
    }
}

impl From<json::Error> for GResourceBuilderError {
    fn from(err: json::Error) -> Self {
        Self::JSON(err, None)
    }
}

impl From<std::string::FromUtf8Error> for GResourceBuilderError {
    fn from(err: FromUtf8Error) -> Self {
        Self::UTF8(err, None)
    }
}

impl Display for GResourceBuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GResourceBuilderError::XMLRead(err, path) => {
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
            GResourceBuilderError::XMLWrite(err, path) => {
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
            GResourceBuilderError::IO(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GResourceBuilderError::JSON(err, path) => {
                if let Some(path) = path {
                    write!(f, "Error parsing JSON from file: {}", path.display())
                } else {
                    write!(f, "Error reading/writing JSON data: {}", err)
                }
            }
            GResourceBuilderError::UTF8(err, path) => {
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

pub type GResourceBuilderResult<T> = Result<T, GResourceBuilderError>;
