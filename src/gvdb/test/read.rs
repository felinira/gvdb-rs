use crate::gvdb::root::GvdbRoot;
use crate::gvdb::test::util::assert_bytes_eq;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use crate::gvdb::root::test::byte_compare_gvdb_file;
#[allow(unused_imports)]
use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};

const TEST_FILE_DIR: &str = "test/data/";
const TEST_FILE_1: &str = "test1.gvdb";
const TEST_FILE_2: &str = "test2.gvdb";
const TEST_FILE_3: &str = "test3.gresource";

fn byte_compare_file(file: &GvdbRoot, reference_filename: &str) {
    let path = PathBuf::from_str(&reference_filename).unwrap();
    let mut reference_file = File::open(path).unwrap();
    let mut reference_data = Vec::new();
    reference_file.read_to_end(&mut reference_data).unwrap();

    assert_bytes_eq(
        &reference_data,
        &file.data(),
        &format!("Byte comparing with file '{}'", reference_filename),
    );
}

pub fn byte_compare_file_1(file: &GvdbRoot) {
    let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
    byte_compare_file(file, &reference_filename);
}

pub fn assert_is_file_1(file: &GvdbRoot) {
    let table = file.hash_table().unwrap();
    let names = table.get_names().unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "root_key");

    let value = table.get_value("root_key").unwrap().child_value(0);
    assert!(value.is_container());
    assert_eq!(value.type_().to_string(), "(uus)");

    assert_eq!(value.child_value(0).get::<u32>().unwrap(), 1234);
    assert_eq!(value.child_value(1).get::<u32>().unwrap(), 98765);
    assert_eq!(
        value.child_value(2).get::<String>().unwrap(),
        "TEST_STRING_VALUE"
    );
}

pub fn byte_compare_file_2(file: &GvdbRoot) {
    let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_2;
    byte_compare_file(file, &reference_filename);
}

pub fn assert_is_file_2(file: &GvdbRoot) {
    let table = file.hash_table().unwrap();
    let names = table.get_names().unwrap();
    assert_eq!(names.len(), 2);
    assert_eq!(names[0], "string");
    assert_eq!(names[1], "table");

    let str_value = table.get_value("string").unwrap().child_value(0);
    assert!(str_value.is_type(glib::VariantTy::STRING));
    assert_eq!(str_value.get::<String>().unwrap(), "test string");

    let sub_table = table.get_hash_table("table").unwrap();
    let sub_table_names = sub_table.get_names().unwrap();
    assert_eq!(sub_table_names.len(), 1);
    assert_eq!(sub_table_names[0], "int");

    let int_value = sub_table.get_value("int").unwrap().child_value(0);
    assert_eq!(int_value.get::<u32>().unwrap(), 42);
}

pub fn byte_compare_file_3(file: &GvdbRoot) {
    let reference_filename = TEST_FILE_DIR.to_string() + TEST_FILE_3;
    let ref_root = GvdbRoot::from_file(&PathBuf::from(reference_filename)).unwrap();
    byte_compare_gvdb_file(&ref_root, file);
}

pub fn assert_is_file_3(file: &GvdbRoot) {
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
    ];
    assert_eq!(names, reference_names);

    let svg1 = table
        .get_value("/gvdb/rs/test/online-symbolic.svg")
        .unwrap()
        .child_value(0);
    let svg1_size = svg1.child_value(0).get::<u32>().unwrap();
    let svg1_flags = svg1.child_value(1).get::<u32>().unwrap();
    let svg1_content = svg1.child_value(2).data_as_bytes();

    assert_eq!(svg1_size, 1390);
    assert_eq!(svg1_flags, 0);
    assert_eq!(svg1_size as usize, svg1_content.len() - 1);

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(svg1_content[svg1_content.len() - 1], 0);
    let svg1_str = std::str::from_utf8(&svg1_content[0..svg1_content.len() - 1]).unwrap();
    assert!(svg1_str.starts_with(
        &(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string()
            + "\n\n"
            + r#"<svg xmlns="http://www.w3.org/2000/svg" height="16px""#)
    ));

    let svg2 = table
        .get_value("/gvdb/rs/test/icons/scalable/actions/send-symbolic.svg")
        .unwrap()
        .child_value(0);
    let svg2_size = svg2.child_value(0).get::<u32>().unwrap();
    let svg2_flags = svg2.child_value(1).get::<u32>().unwrap();
    let svg2_content: &[u8] = &svg2.child_value(2).data_as_bytes();

    assert_eq!(svg2_size, 345);
    assert_eq!(svg2_flags, 1);
    let mut decoder = flate2::read::ZlibDecoder::new(svg2_content);
    let mut svg2_data = Vec::new();
    decoder.read_to_end(&mut svg2_data).unwrap();

    // Ensure the last byte is *not* zero and len is not one bigger than specified because
    // compressed data is not zero-padded
    assert_ne!(svg2_data[svg2_data.len() - 1], 0);
    assert_eq!(svg2_size as usize, svg2_data.len());
    let svg2_str = std::str::from_utf8(&svg2_data).unwrap();

    let mut svg2_reference = String::new();
    File::open("test/data/gresource/icons/scalable/actions/send-symbolic.svg")
        .unwrap()
        .read_to_string(&mut svg2_reference)
        .unwrap();
    assert_str_eq!(svg2_str, svg2_reference);

    let json = table
        .get_value("/gvdb/rs/test/json/test.json")
        .unwrap()
        .child_value(0);
    let json_size = json.child_value(0).get::<u32>().unwrap();
    let json_flags = json.child_value(1).get::<u32>().unwrap();
    let json_content = json.child_value(2).data_as_bytes().to_vec();

    // Ensure the last byte is zero because of zero-padding defined in the format
    assert_eq!(json_content[json_content.len() - 1], 0);
    assert_eq!(json_size as usize, json_content.len() - 1);
    let json_str = std::str::from_utf8(&json_content[0..json_content.len() - 1]).unwrap();

    assert_eq!(json_flags, 0);
    assert_str_eq!(
        json_str,
        r#"{"test":"test_string","int":42,"table":{"bool":true}}"#.to_string() + "\n"
    );
}

#[test]
fn test_file_1() {
    let filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
    let path = PathBuf::from_str(&filename).unwrap();
    let file = GvdbRoot::from_file(&path).unwrap();
    assert_is_file_1(&file);
}

#[test]
fn test_file_2() {
    let filename = TEST_FILE_DIR.to_string() + TEST_FILE_2;
    let path = PathBuf::from_str(&filename).unwrap();
    let file = GvdbRoot::from_file(&path).unwrap();
    assert_is_file_2(&file);
}

#[test]
fn test_file_3() {
    let filename = TEST_FILE_DIR.to_string() + TEST_FILE_3;
    let path = PathBuf::from_str(&filename).unwrap();
    let file = GvdbRoot::from_file(&path).unwrap();
    assert_is_file_3(&file);
}
