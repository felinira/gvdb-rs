mod error;
mod file;
mod hash;
mod item;

pub use error::{Error, Result};
pub use file::{FileWriter, HashTableBuilder};

/// Deprecated type aliases
mod deprecated {
    use super::*;

    /// Type has been renamed. Use [`FileWriter`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::write::FileWriter instead."]
    pub type GvdbFileWriter = FileWriter;

    /// Type has been renamed. Use [`HashTableBuilder`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::write::HashTableBuilder instead."]
    pub type GvdbHashTableBuilder<'a> = HashTableBuilder<'a>;

    /// Type has been renamed. Use [`Error`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::write::Error instead."]
    pub type GvdbWriterError = Error;

    /// Type has been renamed. Use [`Result<T>`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::write::Result<T> instead."]
    pub type GvdbBuilderResult<T> = Result<T>;
}

pub use deprecated::*;
