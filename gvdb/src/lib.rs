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
//!     let path = PathBuf::from("test-data/test3.gresource");
//!     let file = GvdbFile::from_file(&path).unwrap();
//!     let table = file.hash_table().unwrap();
//!
//!     #[derive(serde::Deserialize, zvariant::Type)]
//!     struct SvgData {
//!         size: u32,
//!         flags: u32,
//!         content: Vec<u8>
//!     }
//!    
//!     let value = table
//!         .get_value("/gvdb/rs/test/online-symbolic.svg")
//!         .unwrap();
//!     let structure = value.downcast_ref::<zvariant::Structure>().unwrap();
//!     let svg = structure.fields();
//!     let svg1_size = svg[0].downcast_ref::<u32>().unwrap();
//!     let svg1_flags = svg[1].downcast_ref::<u32>().unwrap();
//!     let svg1_content = svg[2].try_clone().unwrap().downcast::<Vec<u8>>().unwrap();
//!     let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();
//!
//!     println!("{}", svg1_str);
//! }
//! ```
//!
//! Create a simple GVDB file with [`GvdbFileWriter`](crate::write::GvdbFileWriter)
//!
//! ```
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
//!         .insert("int", 42u32)
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
//! ### `mmap`
//!
//! Use the memmap2 crate to read memory-mapped GVDB files.
//!
//! ### `glib`
//!
//! By default this crate uses the [glib](https://crates.io/crates/zvariant) crate to allow reading
//! and writing `GVariant` data to the gvdb files. By enabling this feature you can pass GVariants
//! directly from the glib crate as well.
//!
//! ### `gresource`
//!
//! To be able to compile GResource files, the `gresource` feature must be enabled.
//!
//! ## Macros
//!
//! The [gvdb-macros](https://crates.io/crates/gvdb-macros) crate provides useful macros for
//! GResource file creation.

#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

extern crate core;

/// Read GResource XML files and compile a GResource file
///
/// Use [`GResourceXMLDoc`](crate::gresource::GResourceXMLDocument) for XML file reading and
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
