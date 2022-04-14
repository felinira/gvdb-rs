use crate::gvdb::root::GvdbRoot;
use crate::gvdb::test::util::assert_bytes_eq;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

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

    assert_bytes_eq(&reference_data, &file.data());
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
    byte_compare_file(file, &reference_filename);
}

pub fn assert_is_file_3(file: &GvdbRoot) {
    let table = file.hash_table().unwrap();
    let _names = table.get_names().unwrap();
    let _value = table
        .get_value("/gvdb/rs/test/builder/gvdb-builder.h")
        .unwrap();
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
