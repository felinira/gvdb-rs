# About this crate

This is an implementation of the glib GVariant database file format in Rust. It includes a GResource XML parser and the ability to create compatible GResource files.

[![Crates.io](https://img.shields.io/crates/v/gvdb)](https://crates.io/crates/gvdb)

## MSRV

The minimum supported rust version of this crate is 1.75.

## Breaking changes

### 0.6

This crate now uses zvariant 4.0 and glib 0.19. The MSRV has been increased accordingly.

### 0.5

Added the `mmap` feature, disabled by default.

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
    use gvdb::read::File;

    const GRESOURCE_XML: &str = "test-data/gresource/test3.gresource.xml";

    fn create_gresource() {
        let doc = GResourceXMLDocument::from_file(&PathBuf::from(GRESOURCE_XML)).unwrap();
        let builder = GResourceBuilder::from_xml(doc).unwrap();
        let data = builder.build().unwrap();
        
        // To immediately read this data again, we can create a file reader from the data
        let root = File::from_bytes(Cow::Owned(data)).unwrap();
    }
}
```

Create a simple GVDB file with `FileWriter`

```rust
use gvdb::write::{FileWriter, HashTableBuilder};

fn create_gvdb_file() {
    let mut file_writer = FileWriter::new();
    let mut table_builder = HashTableBuilder::new();
    table_builder
           .insert_string("string", "test string")
           .unwrap();
    let mut table_builder_2 = HashTableBuilder::new();
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
use gvdb::read::File;
use std::path::PathBuf;

pub fn main() {
    let path = PathBuf::from("test-data/test3.gresource");
    let file = File::from_file(&path).unwrap();
    let table = file.hash_table().unwrap();

    #[derive(serde::Deserialize, zvariant::Type)]
    struct GResourceData {
        size: u32,
        flags: u32,
        content: Vec<u8>,
    }

    let svg: GResourceData = table.get("/gvdb/rs/test/online-symbolic.svg").unwrap();

    assert_eq!(svg.size, 1390);
    assert_eq!(svg.flags, 0);
    assert_eq!(svg.size as usize, svg.content.len() - 1);

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(svg.content[svg.content.len() - 1], 0);
    let svg_str = std::str::from_utf8(&svg.content[0..svg.content.len() - 1]).unwrap();

    println!("{}", svg_str);
}
```

## License

`gvdb` and `gvdb-macros` are available under the MIT license. See the [LICENSE.md](./LICENSE.md) file for more info.

SVG icon files included in `test-data/gresource/icons/` are available under the CC0 license and redistributed from [Icon Development Kit](https://gitlab.gnome.org/Teams/Design/icon-development-kit). See the [LICENSE.Icons.md](./LICENSE.Icons.md) and file for more info.
