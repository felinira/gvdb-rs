//! # Read and write GVDB files
//!
//! This crate allows you to read and write GVDB (GLib GVariant database) files.
//! It can also parse GResource XML files and create the corresponding GResource binary
//!
//! ## Examples
//!
//! Load a GResource file from disk with [`GvdbFile`](crate::read::file::GvdbFile)
//!
//! ```
//! use std::path::PathBuf;
//! use gvdb::read::file::GvdbFile;
//!
//! pub fn read_gresource_file() {
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
//!     let mut file_writer = GvdbFileWriter::new();
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
//!     let file_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
//! }
//! ```
//!
//! Create a GResource XML file with [`GResourceXMLDoc`](crate::gresource::xml::GResourceXMLDoc) and
//! [`GResourceBuilder`](crate::gresource::builder::GResourceBuilder)
//! ```
//! use std::borrow::Cow;
//! use std::path::PathBuf;
//! use gvdb::gresource::builder::GResourceBuilder;
//! use gvdb::gresource::xml::GResourceXMLDoc;
//! use gvdb::read::file::GvdbFile;
//!
//! const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";
//!
//! fn create_gresource() {
//!     let doc = GResourceXMLDoc::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
//!     let builder = GResourceBuilder::from_xml(doc).unwrap();
//!     let data = builder.build().unwrap();
//!     let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
//! }
//! ```
//!
//! ## Features
//!
//! To use the GResource XML module, the `gresource` feature must be enabled. This is done by
//! default. You can opt out of the GResource functionality by specifying `default-features = false`
//! in the gvdb dependency declaration
//!
//! ## Macros
//!
//! The [gvdb-macros](https://crates.io/crates/gvdb-macros) crate provides useful macros for
//! GResource file creation.

#![warn(missing_docs)]

/// Read GResource XML files and compile a GResource file
///
/// Use [`GResourceXMLDoc`](crate::gresource::xml::GResourceXMLDoc) for XML file reading and
/// [`GResourceBuilder`](crate::gresource::builder::GResourceBuilder) to create the GResource binary
/// file
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
