use gvdb_macros::{include_gresource_from_dir, include_gresource_from_xml};

#[test]
fn macros() {
    let _data = include_gresource_from_dir!("test", "test-data/gresource");
    let _data2 = include_gresource_from_xml!("test-data/gresource/test3.gresource.xml");
}

#[test]
fn align() {
    for _ in 0..100 {
        let data = include_gresource_from_dir!("test", "test-data/gresource");
        let _u8 = 8u8;
        let ptr_addr = data.as_ptr() as usize;
        assert_eq!(0, ptr_addr % 16);
    }
}
