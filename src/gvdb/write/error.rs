#[derive(Debug)]
pub enum GvdbBuilderError {
    WrongParentPrefix,
    EmptyKey,
    InvalidRootChunk,
    IO(std::io::Error),
    Consistency(String),
}

impl From<std::io::Error> for GvdbBuilderError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

pub type GvdbBuilderResult<T> = std::result::Result<T, GvdbBuilderError>;
