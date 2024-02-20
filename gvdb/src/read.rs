mod error;
mod file;
mod hash;
mod hash_item;
mod header;
mod pointer;

pub use error::{Error, Result};
pub use file::File;
pub use hash::HashTable;

pub(crate) use hash::HashHeader;
pub(crate) use hash_item::{HashItem, HashItemType};
pub(crate) use header::Header;
pub(crate) use pointer::Pointer;

/// Deprecated type aliases
mod deprecated {
    use super::*;

    /// Type has been renamed. Use [`File`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::read::File instead."]
    pub type GvdbFile<'a> = File<'a>;

    /// Type has been renamed. Use [`HashTable`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::read::HashTable instead."]
    pub type GvdbHashTable<'a, 'b> = HashTable<'a, 'b>;

    /// Type has been renamed. Use [`Error`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::read::Error instead."]
    pub type GvdbReaderError = Error;

    /// Type has been renamed. Use [`Result<T>`] instead.
    #[deprecated = "Type has been renamed. Use gvdb::read::Result<T> instead."]
    pub type GvdbReaderResult<T> = Result<T>;
}

pub use deprecated::*;
