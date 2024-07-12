# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The versions in changelog reflect the gvdb versions.

## [0.7.0] - 2024-07-13

### Added

- `gvdb::gresource::BuilderError::StripPrefix`
- `gvdb::gresource::BuilderError::Generic`
- `gvdb::gresource::BundleBuilder::from_directory` now ignores `*.license` files as well

### Removed

- `gvdb::read::GvdbHashTable::for_bytes` (made private)
- `gvdb::read::GvdbHashTable::get_header` (made private)
- `gvdb::read::GvdbHashTable::get_hash_item` (made private)
- `gvdb::read::GvdbReaderError::InvalidData` all instances were replaced with more specific errors as `Data(String)`
- `gvdb::read::GvdbReaderError::ZVariant` all instances were replaced with `Data(String)`
- `gvdb::gresource::GResourceBuilderError::Generic` all instances were replaces with less generic variants

### Changed

- The project and all previous releases are now made available under the MIT OR Apache-2.0 licenses
- The project is now [REUSE compliant](https://reuse.software/)
- Most types have been renamed to remove redundant prefixes and be more consistent with the rest of the Rust ecosystem. The previous names have been added as deprecated type aliases where possible. These aliases will be removed in a future release.
- `gvdb::read` types have gained a few lifetimes. As a result, the reader does not have to borrow the data statically anymore.
- `gvdb::read::HashTable` is no longer `#[repr(C)]` (it was added accidentally)
- `gvdb::gresource::GResourceBuilder` is renamed to `BundleBuilder`
- `gvdb::gresource::GResourceFileData` is renamed to `FileData`
- `gvdb::gresource::GResourceXMLDocument` is renamed to `XmlManifest`
- `gvdb::gresource::GResourceBuilderError` is renamed to `BuilderError` and marked `non_exhaustive`
- `gvdb::gresource::GResourceBuilderResult<T>` is renamed to `BuilderResult<T>`
- `gvdb::gresource::GResourceXMLError` is renamed to `XmlManifestError` and marked `non_exhaustive`
- `gvdb::gresource::GResourceXMLResult<T>` is renamed to `XmlManifestResult<T>`
- `gvdb::read::GvdbFile` is renamed to `File`
- `gvdb::read::GvdbHashTable` is renamed to `HashTable`
- `gvdb::read::HashTable::get_names` is renamed to `keys`
- `gvdb::read::GvdbReaderError` is renamed to `Error` and marked `non_exhaustive`
- `gvdb::read::Error::DataError` is renamed to `Error::Data`
- `gvdb::read::Error::KeyError` is renamed to `Error::KeyNotFound`
- `gvdb::read::Error::Utf8` now uses `std::str::Utf8Error` instead of `std::string::FromUtf8Error` and is marked `non_exhaustive`
- `gvdb::read::GvdbReaderResult<T>` is renamed to `Result<T>`
- `gvdb::write::GvdbFileWriter` is renamed to `FileWriter`
- `gvdb::write::GvdbHashTableBuilder` is renamed to `HashTableBuilder`
- `gvdb::write::GvdbWriterError` is renamed to `Error` and marked `non_exhaustive`
- `gvdb::write::GvdbBuilderResult<T>` is renamed to `Result<T>`

## [0.6.1] - 2024-02-23

### Changed

- gvdb: Fix compilation on non-unix platforms

## [0.6.0] - 2024-02-22

### Changed

- Upgrade dependencies: zvariant 4.0 and glib 0.19. The MSRV has been increased accordingly.

## [0.5.3] - 2023-12-02

### Changed

- Upgrade dependencies: quick-xml 0.31
- Update MSRV to 1.70

## [0.5.2] - 2023-08-30

### Added

- Custom debug implementation for `GvdbHashTable`
- More Tests

### Fixed

- Invalid endianess of `GvdbPointer.key_size`

## [0.5.1] - 2023-07-29

### Changed

- Upgrade dependencies: quick-xml 0.30

## [0.5.0] - 2023-06-21

### Added

- Make `mmap` feature optional, disabled by default.

## [0.4.2] - 2023-06-21

### Changed

- Upgrade dependencies: memmap2 0.7

## [0.4.1] - 2023-05-13

### Changed

- `gvdb::gresource::GResourceBuilder::from_directory` now ignores `.gitignore` files

## [0.4.0] - 2023-03-06

### Added

- Improve test coverage for writer

### Changed

- dependencies: Replace json with serde_json
- dependencies: Replace xml-rs with quick-xml
- dependencies: Update glib

### Fixes

- Make test3 idempotent

## [0.3.0] - 2022-10-22

### Added

- Hugely improve test coverage for all modules

### Changed

- Byteswap test file 2 to better test for big endian files

## [0.2.2] - 2022-06-15

### Added

- Implement `std::error::Error` for error types #1
- Expose `GResourceFileData` API #2

## [0.2.1] - 2022-05-26

### Changed

- Build docs.rs documentation with all features enabled

## [0.2.0] - 2022-05-17

### Changed

- Use zvariant for value serialisation instead of glib

## [0.1.3] - 2022-05-12

### Fixes

- `GvdbFileWriter`:  Ensure that an empty bucket at the end would still get the correct data 

## [0.1.2] - 2022-04-22

### Changed

- Remove lifetime from `GvdbFile`

## [0.1.1] - 2022-04-22

### Added

- `gvdb::read::GvdbFile::from_file_mmap`

### Changed

- default-features does not include the gresource feature anymore

## [0.1.0] - 2022-04-21

### Added

- Initial release