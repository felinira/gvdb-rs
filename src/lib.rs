//! # Read and write GVDB files
//!
//! This crate allows you to read and write GVDB (GLib GVariant database) files.
//! It can also parse GResource XML files and create the corresponding GResource binary
//!
//! ## Examples
//!
//! Load a GResource file from disk with [`GvdbFile`](crate::read::GvdbFile)
//!
//! ```
//! use std::path::PathBuf;
//! use gvdb::read::GvdbFile;
//!
//! pub fn read_gresource_file() {
//!     let path = PathBuf::from("test/data/test3.gresource");
//!     let file = GvdbFile::from_file(&path).unwrap();
//!     let table = file.hash_table().unwrap();
//!
//!     let svg1 = table
//!         .get_gvariant("/gvdb/rs/test/online-symbolic.svg")
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
//! Create a simple GVDB file with [`GvdbFileWriter`](crate::write::GvdbFileWriter)
//!
//! ```
//! #[cfg(feature = "glib")]
//! # use glib::ToVariant;
//! use gvdb::write::{GvdbFileWriter, GvdbHashTableBuilder};
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
//!         .insert_gvariant("int", 42u32.to_variant())
//!         .unwrap();
//!
//!     table_builder
//!         .insert_table("table", table_builder_2)
//!         .unwrap();
//!     let file_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
//! }
//! ```
//!
//! ## Features
//!
//! By default, no features are enabled.
//!
//! ### `glib`
//!
//! By default this crate uses the [glib](https://crates.io/crates/zvariant) crate to allow reading
//! and writing `GVariant` data to the gvdb files. By enabling this feature you can pass GVariants
//! directly from the glib crate as well.
//!
//! ### `gresource`
//!
//! To use the GResource compilation module, the `gresource` feature must be enabled.
//!
//! ## Macros
//!
//! The [gvdb-macros](https://crates.io/crates/gvdb-macros) crate provides useful macros for
//! GResource file creation.

#![warn(missing_docs)]

extern crate core;

/// Read GResource XML files and compile a GResource file
///
/// Use [`GResourceXMLDoc`](crate::gresource::GResourceXMLDoc) for XML file reading and
/// [`GResourceBuilder`](crate::gresource::GResourceBuilder) to create the GResource binary
/// file
#[cfg(feature = "gresource")]
pub mod gresource;

/// Read GVDB files from a file or from a byte slice
///
/// See the documentation of [`GvdbFile`](crate::read::GvdbFile) to get started
pub mod read;

/// Create GVDB files
///
/// See the documentation of [`GvdbFileWriter`](crate::write::GvdbFileWriter) to get started
pub mod write;

#[cfg(test)]
pub(crate) mod test;

mod util;
