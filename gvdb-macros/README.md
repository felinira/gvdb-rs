# About this crate

This crate offers convenience macros for [gvdb](https://crates.io/crates/gvdb).
The macros are `include_gresource_from_xml!()` and `include_gresource_from_dir!()`

[![Crates.io](https://img.shields.io/crates/v/gvdb-macros)](https://crates.io/crates/gvdb-macros)

## Examples

Compile a GResource XML file and include the bytes in the file.

```rust
use gvdb_macros::include_gresource_from_xml;
static GRESOURCE_BYTES: &[u8] = include_gresource_from_xml!("test-data/gresource/test3.gresource.xml");
```

Scan a directory and create a GResource file with all the contents of the directory.

```rust
use gvdb_macros::include_gresource_from_dir;
static GRESOURCE_BYTES: &[u8] = include_gresource_from_dir!("/gvdb/rs/test", "test-data/gresource/");
```
