//! # Read and write GVDB files
//!
//! This crate allows you to read and write GVDB (GLib GVariant database) files.
//! It can also parse GResource XML files and create the corresponding GResource binary
//!
//! ## Examples
//!
//! Example: Load a GResource file from disk with [`GvdbFile`](crate::read::file::GvdbFile)
//!
//! ```
//! use std::path::PathBuf;
//! use gvdb::read::file::GvdbFile;
//!
//! pub fn main() {
//!     let path = PathBuf::from("test/data/test3.gresource");
//!     let file = GvdbFile::from_file(&path).unwrap();
//!     let table = file.hash_table().unwrap();
//!
//!     let svg1 = table
//!         .get_value("/gvdb/rs/test/online-symbolic.svg")
//!         .unwrap()
//!         .child_value(0);
//!     let svg1_size = svg1.child_value(0).get::<u32>().unwrap();
//!     let svg1_flags = svg1.child_value(1).get::<u32>().unwrap();
//!     let svg1_content = svg1.child_value(2).data_as_bytes();
//!     let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();
//!
//!     println!("{}", svg1_str);
//! }
//! ```
//!
//! Create a simple GVDB file with [`GvdbFileWriter`](crate::write::file::GvdbFileWriter)
//!
//! ```
//! use glib::ToVariant;
//! use gvdb::write::file::{GvdbFileWriter, GvdbHashTableBuilder};
//!
//! fn create_gvdb_file() {
//!     let mut file_builder = GvdbFileWriter::new(false);
//!     let mut table_builder = GvdbHashTableBuilder::new();
//!     table_builder
//!            .insert_string("string", "test string")
//!            .unwrap();
//!
//!     let mut table_builder_2 = GvdbHashTableBuilder::new();
//!     table_builder_2
//!         .insert_variant("int", 42u32.to_variant())
//!         .unwrap();
//!
//!     table_builder
//!         .insert_table("table", table_builder_2)
//!         .unwrap();
//!     let root_index = file_builder.write_into_vec_with_table(table_builder).unwrap();
//! }
//! ```

extern crate core;

#[cfg(feature = "gresource")]
pub mod gresource;

/// Read GVDB files from a file or from a byte slice
///
/// See the documentation of [`GvdbFile`](crate::read::file::GvdbFile) to get started
pub mod read;

/// Create GVDB files
pub mod write;

#[cfg(test)]
pub(crate) mod test;

mod util;
