//! # Read and write GVDB files
//!
//! This crate allows you to read and write GVDB (GLib GVariant database) files.
//! It can also parse GResource XML files and create the corresponding GResource binary
//!
//! ## Examples
//!
//! Load a GResource file from disk with [`File`](crate::read::File)
//!
//! ```
//! use std::path::PathBuf;
//! use gvdb::read::File;
//!
//! pub fn read_gresource_file() {
//!     let path = PathBuf::from("test-data/test3.gresource");
//!     let file = File::from_file(&path).unwrap();
//!     let table = file.hash_table().unwrap();
//!
//!     #[derive(serde::Deserialize, zvariant::Type)]
//!     struct SvgData {
//!         size: u32,
//!         flags: u32,
//!         content: Vec<u8>
//!     }
//!    
//!     let svg: SvgData = table
//!         .get("/gvdb/rs/test/online-symbolic.svg")
//!         .unwrap();
//!     let svg_str = std::str::from_utf8(&svg.content).unwrap();
//!
//!     println!("{}", svg_str);
//! }
//! ```
//!
//! Create a simple GVDB file with [`FileWriter`](crate::write::FileWriter)
//!
//! ```
//! use gvdb::write::{FileWriter, HashTableBuilder};
//!
//! fn create_gvdb_file() {
//!     let mut file_writer = FileWriter::new();
//!     let mut table_builder = HashTableBuilder::new();
//!     table_builder
//!            .insert_string("string", "test string")
//!            .unwrap();
//!
//!     let mut table_builder_2 = HashTableBuilder::new();
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
#![allow(unknown_lints, clippy::assigning_clones)]
#![doc = include_str!("../README.md")]

extern crate core;

/// Read GResource XML files and compile a GResource file
///
/// Use [`XmlManifest`](crate::gresource::XmlManifest) for XML file reading and
/// [`BundleBuilder`](crate::gresource::BundleBuilder) to create the GResource binary
/// file
#[cfg(feature = "gresource")]
pub mod gresource;

/// Read GVDB files from a file or from a byte slice
///
/// See the documentation of [`File`](crate::read::File) to get started
pub mod read;

/// Create GVDB files
///
/// See the documentation of [`FileWriter`](crate::write::FileWriter) to get started
pub mod write;

/// Serialize types as GVariant
pub mod variant;

#[cfg(test)]
pub(crate) mod test;

mod endian;
mod util;

pub(crate) use endian::Endian;
