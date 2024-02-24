# About these crates

This repository contains the crates [gvdb](https://github.com/felinira/gvdb-rs/blob/main/gvdb) and [gvdb-macros](https://github.com/felinira/gvdb-rs/blob/main/gvdb-macros).

[![GitHub](https://img.shields.io/github/license/felinira/gvdb-rs)](https://github.com/felinira/gvdb-rs/blob/main/LICENSE.md)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/felinira/gvdb-rs/ci.yml?branch=main)](https://github.com/felinira/gvdb-rs/actions/workflows/ci.yml)
[![Codecov](https://img.shields.io/codecov/c/github/felinira/gvdb-rs?token=YDF2YPLDIK)](https://codecov.io/gh/felinira/gvdb-rs)

## gvdb

[![Crates.io](https://img.shields.io/crates/v/gvdb)](https://crates.io/crates/gvdb)

This is an implementation of the glib GVariant database file format in Rust. It includes a GResource XML parser and the ability to create compatible GResource files.

## gvdb-macros

[![Crates.io](https://img.shields.io/crates/v/gvdb-macros)](https://crates.io/crates/gvdb-macros)

This crate offers convenience macros for [gvdb](https://crates.io/crates/gvdb).
The macros are `include_gresource_from_xml!()` and `include_gresource_from_dir!()`

## License

`gvdb` and `gvdb-macros` are available under the MIT OR Apache-2.0 license. See the [LICENSES](./LICENSES) folder for the complete license text.

SVG icon files included in `test-data/gresource/icons/` are available under the CC0-1.0 license and redistributed from [Icon Development Kit](https://gitlab.gnome.org/Teams/Design/icon-development-kit). See [CC0-1.0.txt](./LICENSES/CC0-1.0.txt) and file for complete license text.
