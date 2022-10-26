# About this crate

This is a re-implementation of the glib GVariant database file format in Rust. It includes a GResource XML parser.

[![CI](https://github.com/felinira/gvdb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/felinira/gvdb-rs/actions/workflows/ci.yml)

## MSRV

The minimal rust version of this crate is 1.65.

## Example

### Create a GResource file

Create a GResource file from XML with `GResourceXMLDocument` and `GResourceBuilder`.

Requires the `gresource` feature to be enabled.

```rust
#[cfg(feature = "gresource")]
mod gresource {
    use std::borrow::Cow;
    use std::path::PathBuf;
    use gvdb::gresource::GResourceBuilder;
    use gvdb::gresource::GResourceXMLDocument;
    use gvdb::read::GvdbFile;

    const GRESOURCE_XML: &str = "test/data/gresource/test3.gresource.xml";

    fn create_gresource() {
        let doc = GResourceXMLDocument::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
        let builder = GResourceBuilder::from_xml(doc).unwrap();
        let data = builder.build().unwrap();
        
        // To immediately read this data again, we can create a file reader from the data
        let root = GvdbFile::from_bytes(Cow::Owned(data)).unwrap();
    }
}
```

Create a simple GVDB file with `GvdbFileWriter`

```rust
use gvdb::write::{GvdbFileWriter, GvdbHashTableBuilder};

fn create_gvdb_file() {
    let mut file_writer = GvdbFileWriter::new();
    let mut table_builder = GvdbHashTableBuilder::new();
    table_builder
           .insert_string("string", "test string")
           .unwrap();
    let mut table_builder_2 = GvdbHashTableBuilder::new();
    table_builder_2
        .insert("int", 42u32)
        .unwrap();

    table_builder
        .insert_table("table", table_builder_2)
        .unwrap();
    let file_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
}
```

### Read a GVDB file

The stored data at `/gvdb/rs/test/online-symbolic.svg` corresponds to the `(uuay)` GVariant type signature.

```rust
use gvdb::read::GvdbFile;
use std::path::PathBuf;

pub fn main() {
    let path = PathBuf::from("test/data/test3.gresource");
    let file = GvdbFile::from_file(&path).unwrap();
    let table = file.hash_table().unwrap();

    #[derive(zvariant::OwnedValue)]
    struct GResourceData {
        size: u32,
        flags: u32,
        content: Vec<u8>,
    }

    let svg1: GResourceData = table.get("/gvdb/rs/test/online-symbolic.svg").unwrap();

    assert_eq!(svg1.size, 1390);
    assert_eq!(svg1.flags, 0);
    assert_eq!(svg1.size as usize, svg1.content.len() - 1);

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(svg1.content[svg1.content.len() - 1], 0);
    let svg1_str = std::str::from_utf8(&svg1.content[0..svg1.content.len() - 1]).unwrap();

    println!("{}", svg1_str);
}
```
