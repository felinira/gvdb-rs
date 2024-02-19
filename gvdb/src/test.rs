#![allow(unused)]

use crate::read::{File, HashItemType, HashTable};
use crate::write::{GvdbFileWriter, GvdbHashTableBuilder};
use lazy_static::lazy_static;
pub use matches::assert_matches;
pub use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};
use std::borrow::Cow;
use std::cmp::{max, min};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

lazy_static! {
    pub(crate) static ref TEST_FILE_DIR: PathBuf = PathBuf::from("test-data");
    pub(crate) static ref TEST_FILE_1: PathBuf = TEST_FILE_DIR.join("test1.gvdb");
    pub(crate) static ref TEST_FILE_2: PathBuf = TEST_FILE_DIR.join("test2.gvdb");
    pub(crate) static ref TEST_FILE_3: PathBuf = TEST_FILE_DIR.join("test3.gresource");
    pub(crate) static ref GRESOURCE_DIR: PathBuf = TEST_FILE_DIR.join("gresource");
    pub(crate) static ref GRESOURCE_XML: PathBuf = GRESOURCE_DIR.join("test3.gresource.xml");
}

fn write_byte_row(
    f: &mut dyn std::io::Write,
    offset: usize,
    bytes_per_row: usize,
    bytes: &[u8],
) -> std::io::Result<()> {
    write!(f, "{:08X}", offset)?;

    for (index, byte) in bytes.iter().enumerate() {
        if index % 4 == 0 {
            write!(f, " ")?;
        }

        write!(f, " {:02X}", byte)?;
    }

    let bytes_per_row = max(bytes_per_row, bytes.len());
    for index in bytes.len()..bytes_per_row {
        if index % 4 == 0 {
            write!(f, " ")?;
        }

        write!(f, "   ")?;
    }

    write!(f, "  ")?;

    for byte in bytes {
        if byte.is_ascii_alphanumeric() || byte.is_ascii_whitespace() {
            write!(f, "{}", *byte as char)?;
        } else {
            write!(f, ".")?;
        }
    }

    writeln!(f)
}

fn write_byte_rows(
    f: &mut dyn std::io::Write,
    center_offset: usize,
    additional_rows_top: usize,
    additional_rows_bottom: usize,
    bytes_per_row: usize,
    bytes: &[u8],
) -> std::io::Result<()> {
    let center_row_num = center_offset / bytes_per_row;
    let start_row = center_row_num - min(center_row_num, additional_rows_top);
    // We add 1 because we can add partial rows at the end
    let last_row = min(
        additional_rows_bottom + center_row_num,
        bytes.len() / bytes_per_row + 1,
    );
    let row_count = last_row - start_row;

    for row in 0..row_count {
        let offset_start = (start_row + row) * bytes_per_row;
        let offset_end = min(bytes.len(), offset_start + bytes_per_row);

        write_byte_row(
            f,
            offset_start,
            bytes_per_row,
            &bytes[offset_start..offset_end],
        )?;
    }

    Ok(())
}

pub fn assert_bytes_eq(a: &[u8], b: &[u8], context: &str) {
    const WIDTH: usize = 16;
    const EXTRA_ROWS_TOP: usize = 8;
    const EXTRA_ROWS_BOTTOM: usize = 4;

    let max_len = max(a.len(), b.len());

    for index in 0..max_len {
        let a_byte = a.get(index);
        let b_byte = b.get(index);

        if a_byte.is_none() || b_byte.is_none() || a_byte.unwrap() != b_byte.unwrap() {
            let mut a_bytes_buf = Vec::new();
            write_byte_rows(
                &mut a_bytes_buf,
                index,
                EXTRA_ROWS_TOP,
                EXTRA_ROWS_BOTTOM,
                WIDTH,
                &a,
            )
            .unwrap();
            let str_a = String::from_utf8(a_bytes_buf).unwrap();

            let mut b_bytes_buf = Vec::new();
            write_byte_rows(
                &mut b_bytes_buf,
                index,
                EXTRA_ROWS_TOP,
                EXTRA_ROWS_BOTTOM,
                WIDTH,
                &b,
            )
            .unwrap();
            let str_b = String::from_utf8(b_bytes_buf).unwrap();

            eprintln!("{}", context);
            assert_str_eq!(str_a, str_b);
        }
    }
}

pub fn byte_compare_gvdb_file(a: &File, b: &File) {
    assert_eq!(a.get_header().unwrap(), b.get_header().unwrap());

    let a_hash = a.hash_table().unwrap();
    let b_hash = b.hash_table().unwrap();
    byte_compare_gvdb_hash_table(&a_hash, &b_hash);
}

fn byte_compare_file(file: &File, reference_path: &Path) {
    let mut reference_file = std::fs::File::open(reference_path).unwrap();
    let mut reference_data = Vec::new();
    reference_file.read_to_end(&mut reference_data).unwrap();

    assert_bytes_eq(
        &reference_data,
        file.data.as_ref(),
        &format!("Byte comparing with file '{}'", reference_path.display()),
    );
}

pub fn byte_compare_file_1(file: &File) {
    byte_compare_file(file, &TEST_FILE_1);
}

pub fn assert_is_file_1(file: &File) {
    let table = file.hash_table().unwrap();
    let names = table.get_names().unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "root_key");

    let value = table.get_value("root_key").unwrap();
    assert_matches!(value, zvariant::Value::Structure(_));
    assert_eq!(value.value_signature(), "(uus)");

    let tuple = zvariant::Structure::try_from(value).unwrap();
    let fields = tuple.into_fields();

    assert_eq!(u32::try_from(&fields[0]), Ok(1234));
    assert_eq!(u32::try_from(&fields[1]), Ok(98765));
    assert_eq!(<&str>::try_from(&fields[2]), Ok("TEST_STRING_VALUE"));
}

pub fn byte_compare_file_2(file: &File) {
    byte_compare_file(file, &TEST_FILE_2);
}

pub fn assert_is_file_2(file: &File) {
    let table = file.hash_table().unwrap();
    let names = table.get_names().unwrap();
    assert_eq!(names.len(), 2);
    assert_eq!(names[0], "string");
    assert_eq!(names[1], "table");

    let str_value = table.get_value("string").unwrap();
    assert_matches!(str_value, zvariant::Value::Str(_));
    assert_eq!(<&str>::try_from(&str_value), Ok("test string"));

    let sub_table = table.get_hash_table("table").unwrap();
    let sub_table_names = sub_table.get_names().unwrap();
    assert_eq!(sub_table_names.len(), 1);
    assert_eq!(sub_table_names[0], "int");

    let int_value = sub_table.get_value("int").unwrap();
    assert_eq!(u32::try_from(int_value), Ok(42));
}

pub fn byte_compare_file_3(file: &File) {
    let ref_root = File::from_file(&TEST_FILE_3).unwrap();
    byte_compare_gvdb_file(file, &ref_root);
}

pub fn assert_is_file_3(file: &File) {
    let table = file.hash_table().unwrap();
    let mut names = table.get_names().unwrap();
    names.sort();
    let reference_names = vec![
        "/",
        "/gvdb/",
        "/gvdb/rs/",
        "/gvdb/rs/test/",
        "/gvdb/rs/test/icons/",
        "/gvdb/rs/test/icons/scalable/",
        "/gvdb/rs/test/icons/scalable/actions/",
        "/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg",
        "/gvdb/rs/test/json/",
        "/gvdb/rs/test/json/test.json",
        "/gvdb/rs/test/online-symbolic.svg",
        "/gvdb/rs/test/test.css",
    ];
    assert_eq!(names, reference_names);

    #[derive(Clone, zvariant::Type, serde::Deserialize)]
    struct GResourceData {
        size: u32,
        flags: u32,
        content: Vec<u8>,
    }

    let svg1: GResourceData = table
        .get::<GResourceData>("/gvdb/rs/test/online-symbolic.svg")
        .unwrap();

    assert_eq!(svg1.size, 1390);
    assert_eq!(svg1.flags, 0);
    assert_eq!(svg1.size as usize, svg1.content.len() - 1);

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(svg1.content[svg1.content.len() - 1], 0);
    let svg1_str = std::str::from_utf8(&svg1.content[0..svg1.content.len() - 1]).unwrap();
    assert!(svg1_str.starts_with(
        &(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string()
            + "\n\n"
            + r#"<svg xmlns="http://www.w3.org/2000/svg" height="16px""#)
    ));

    let svg2 = table
        .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
        .unwrap();
    assert_matches!(svg2, zvariant::Value::Structure(_));
    let svg2_fields = zvariant::Structure::try_from(svg2).unwrap().into_fields();

    let svg2_size = u32::try_from(&svg2_fields[0]).unwrap();
    let svg2_flags = u32::try_from(&svg2_fields[1]).unwrap();
    let svg2_content: Vec<u8> = <Vec<u8>>::try_from(svg2_fields[2].try_clone().unwrap()).unwrap();

    assert_eq!(svg2_size, 345);
    assert_eq!(svg2_flags, 1);
    let mut decoder = flate2::read::ZlibDecoder::new(&*svg2_content);
    let mut svg2_data = Vec::new();
    decoder.read_to_end(&mut svg2_data).unwrap();

    // Ensure the last byte is *not* zero and len is not one bigger than specified because
    // compressed data is not zero-padded
    assert_ne!(svg2_data[svg2_data.len() - 1], 0);
    assert_eq!(svg2_size as usize, svg2_data.len());
    let svg2_str = std::str::from_utf8(&svg2_data).unwrap();

    let mut svg2_reference = String::new();
    std::fs::File::open(GRESOURCE_DIR.join("icons/scalable/actions/send-symbolic.svg"))
        .unwrap()
        .read_to_string(&mut svg2_reference)
        .unwrap();
    assert_str_eq!(svg2_str, svg2_reference);

    let json =
        zvariant::Structure::try_from(table.get_value("/gvdb/rs/test/json/test.json").unwrap())
            .unwrap()
            .into_fields();
    let json_size: u32 = (&json[0]).try_into().unwrap();
    let json_flags: u32 = (&json[1]).try_into().unwrap();
    let json_content: Vec<u8> = json[2].try_clone().unwrap().try_into().unwrap();

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(json_content[json_content.len() - 1], 0);
    assert_eq!(json_size as usize, json_content.len() - 1);
    let json_str = std::str::from_utf8(&json_content[0..json_content.len() - 1]).unwrap();

    assert_eq!(json_flags, 0);
    assert_str_eq!(
        json_str,
        r#"["test_string",42,{"bool":true}]"#.to_string() + "\n"
    );
}

pub(crate) fn new_empty_file() -> File<'static> {
    let writer = GvdbFileWriter::new();
    let table_builder = GvdbHashTableBuilder::new();
    let data = Vec::new();
    let mut cursor = Cursor::new(data);
    writer.write_with_table(table_builder, &mut cursor).unwrap();

    File::from_bytes(Cow::Owned(cursor.into_inner())).unwrap()
}

pub(crate) fn new_simple_file(big_endian: bool) -> File<'static> {
    let writer = if big_endian {
        GvdbFileWriter::for_big_endian()
    } else {
        GvdbFileWriter::new()
    };

    let mut table_builder = GvdbHashTableBuilder::new();
    table_builder.insert("test", "test").unwrap();
    let data = Vec::new();
    let mut cursor = Cursor::new(data);
    writer.write_with_table(table_builder, &mut cursor).unwrap();

    File::from_bytes(Cow::Owned(cursor.into_inner())).unwrap()
}

pub(crate) fn byte_compare_gvdb_hash_table(a: &HashTable, b: &HashTable) {
    assert_eq!(a.get_header(), b.get_header());

    let mut keys_a = a.get_names().unwrap();
    let mut keys_b = b.get_names().unwrap();
    keys_a.sort();
    keys_b.sort();
    assert_eq!(keys_a, keys_b);

    for key in keys_a {
        let item_a = a.get_hash_item(&key).unwrap();
        let item_b = b.get_hash_item(&key).unwrap();

        assert_eq!(item_a.hash_value(), item_b.hash_value());
        assert_eq!(item_a.key_size(), item_b.key_size());
        assert_eq!(item_a.typ().unwrap(), item_b.typ().unwrap());
        assert_eq!(item_a.value_ptr().size(), item_b.value_ptr().size());

        let data_a = a.file.dereference(item_a.value_ptr(), 1).unwrap();
        let data_b = b.file.dereference(item_b.value_ptr(), 1).unwrap();

        // We don't compare containers, only their length
        if item_a.typ().unwrap() == HashItemType::Container {
            if data_a.len() != data_b.len() {
                // The lengths should not be different. For context we will compare the data
                assert_bytes_eq(
                    data_a,
                    data_b,
                    &format!("Containers with key '{}' have different lengths", key),
                );
            }
        } else {
            assert_bytes_eq(
                data_a,
                data_b,
                &format!("Comparing items with key '{}'", key),
            );
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn assert_bytes_eq1() {
        super::assert_bytes_eq(&[1, 2, 3], &[1, 2, 3], "test");
    }

    #[test]
    fn assert_bytes_eq2() {
        // b is exactly 16 bytes long to test "b is too small" panic
        super::assert_bytes_eq(
            b"help i am stuck in a test case",
            b"help i am stuck in a test case",
            "test",
        );
    }

    #[test]
    #[should_panic]
    fn assert_bytes_eq_fail1() {
        super::assert_bytes_eq(&[1, 2, 4], &[1, 2, 3], "test");
    }

    #[test]
    #[should_panic]
    fn assert_bytes_eq_fail2() {
        super::assert_bytes_eq(&[1, 2, 3, 4], &[1, 2, 3], "test");
    }

    #[test]
    #[should_panic]
    fn assert_bytes_eq_fail3() {
        super::assert_bytes_eq(&[1, 2, 3], &[1, 2, 3, 4], "test");
    }

    #[test]
    #[should_panic]
    fn assert_bytes_eq_fail4() {
        // b is exactly 16 bytes long to test "b is too small" panic
        super::assert_bytes_eq(
            b"help i am stuck in a test case",
            b"help i am stuck in a test cas",
            "test",
        );
    }
}
