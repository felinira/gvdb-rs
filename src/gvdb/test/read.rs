use crate::gvdb::root::GvdbRoot;
use std::path::PathBuf;
use std::str::FromStr;

const TEST_FILE_DIR: &str = "test/data/";
const TEST_FILE_1: &str = "test1.gvdb";

#[test]
pub fn test_file_1() {
    let filename = TEST_FILE_DIR.to_string() + TEST_FILE_1;
    let path = PathBuf::from_str(&filename).unwrap();
    let file = GvdbRoot::from_file(&path).unwrap();

    let table = file.get_hash_table_root().unwrap();
    let names = table.get_names().unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "root_key");

    let value = table.get_value("root_key").unwrap();
    println!("{}", value);
    assert!(value.is_container());
    assert!(value.is_type(glib::VariantTy::VARIANT));

    let inner = value.child_value(0);
    assert_eq!(inner.type_().to_string(), "(uus)");
}
