/// Error when parsing a GResource XML file
pub enum XmlManifestError {
    /// An error occured during parsing of the XML file
    Serde(quick_xml::de::DeError, Option<std::path::PathBuf>),

    /// Generic I/O error occurred when handling XML file
    Io(std::io::Error, Option<std::path::PathBuf>),

    /// A file needs to be interpreted as UTF-8 (for stripping whitespace etc.) but it is invalid
    Utf8(std::str::Utf8Error, Option<std::path::PathBuf>),
}

impl XmlManifestError {
    pub(crate) fn from_io_with_filename(
        filename: &std::path::Path,
    ) -> impl FnOnce(std::io::Error) -> XmlManifestError {
        let path = filename.to_path_buf();
        move |err| XmlManifestError::Io(err, Some(path))
    }
}

impl std::error::Error for XmlManifestError {}

impl std::fmt::Display for XmlManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlManifestError::Serde(err, path) => {
                if let Some(path) = path {
                    write!(f, "Error parsing XML file '{}': {}", path.display(), err)
                } else {
                    write!(f, "Error parsing XML file: {}", err)
                }
            }
            XmlManifestError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            XmlManifestError::Utf8(err, path) => {
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
        }
    }
}

impl std::fmt::Debug for XmlManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Result type for XmlManifestError
pub type XmlManifestResult<T> = std::result::Result<T, XmlManifestError>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from() {
        let io_res = std::fs::File::open("test/invalid_file_name");
        let err = XmlManifestError::Io(io_res.unwrap_err(), None);
        assert!(format!("{}", err).contains("I/O"));
    }
}
