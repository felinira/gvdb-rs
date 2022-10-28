mod builder;
mod error;
mod xml;

pub use self::xml::{GResourceXMLDocument, PreprocessOptions};
pub use builder::{GResourceBuilder, GResourceFileData};
pub use error::{
    GResourceBuilderError, GResourceBuilderResult, GResourceXMLError, GResourceXMLResult,
};
