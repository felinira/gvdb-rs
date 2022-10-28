use gvdb_macros::{include_gresource_from_dir, include_gresource_from_xml};

#[test]
fn macros() {
    let _data = include_gresource_from_dir!("test", "test-data/gresource");
    let _data2 = include_gresource_from_xml!("test-data/gresource/test3.gresource.xml");
}
