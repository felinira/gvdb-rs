use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;

pub enum GvdbWriterError {
    Io(std::io::Error, Option<PathBuf>),
    Consistency(String),
}

impl From<std::io::Error> for GvdbWriterError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err, None)
    }
}

impl Display for GvdbWriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GvdbWriterError::Io(err, path) => {
                if let Some(path) = path {
                    write!(f, "I/O error for file '{}': {}", path.display(), err)
                } else {
                    write!(f, "I/O error: {}", err)
                }
            }
            GvdbWriterError::Consistency(context) => {
                write!(f, "Internal inconsistency: {}", context)
            }
        }
    }
}

impl Debug for GvdbWriterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

pub type GvdbBuilderResult<T> = std::result::Result<T, GvdbWriterError>;
