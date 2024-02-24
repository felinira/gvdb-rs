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

## License

`gvdb` and `gvdb-macros` are available under the MIT OR Apache-2.0 license. See the [LICENSES](./LICENSES) folder for the complete license text.

SVG icon files included in `test-data/gresource/icons/` are available under the CC0-1.0 license and redistributed from [Icon Development Kit](https://gitlab.gnome.org/Teams/Design/icon-development-kit). See [CC0-1.0.txt](./LICENSES/CC0-1.0.txt) and file for complete license text.
