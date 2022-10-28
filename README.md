# About these crates

This repository contains the crates [gvdb](https://github.com/felinira/gvdb-rs/blob/main/gvdb) and [gvdb-macros](https://github.com/felinira/gvdb-rs/blob/main/gvdb-macros).

## gvdb

This is an implementation of the glib GVariant database file format in Rust. It includes a GResource XML parser and the ability to create compatible GResource files.

## gvdb-macros

This crate offers convenience macros for [gvdb](https://crates.io/crates/gvdb).
The macros are `include_gresource_from_xml!()` and `include_gresource_from_dir!()`
