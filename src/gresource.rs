mod builder;
mod error;
mod xml;

pub use self::xml::GResourceXMLDocument;
pub use builder::GResourceBuilder;
pub use error::{
    GResourceBuilderError, GResourceBuilderResult, GResourceXMLError, GResourceXMLResult,
};
