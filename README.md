# About this crate

This is a re-implementation of the glib GVariant database file format in Rust. It includes a GResource XML parser.

## Example

### Create a GResource file

```rust
use gvdb::write::builder::GvdbBuilder;

pub fn main() {
    let path = PathBuf::from("test/data/gresource/test3.gresource.xml");
    let xml = gvdb::gresource::xml::GResourceXMLDoc::from_file(&path).unwrap();
    let builder = gvdb::gresource::builder::GResourceBuilder::from_xml(xml).unwrap();
    let data = builder.build().unwrap();
}
```

### Read a GResource XML file

```rust
use gvdb::read::file::GvdbFile;

pub fn main() {
    let path = PathBuf::from("test/data/test3.gresource");
    let file = GvdbFile::from_file(&path).unwrap();

    let svg1 = table
        .get_value("/gvdb/rs/test/online-symbolic.svg")
        .unwrap()
        .child_value(0);
    let svg1_size = svg1.child_value(0).get::<u32>().unwrap();
    let svg1_flags = svg1.child_value(1).get::<u32>().unwrap();
    let svg1_content = svg1.child_value(2).data_as_bytes();
    let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();

    println!("{}", svg1_str);
}
```