use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use safe_transmute::{transmute_one_pedantic, transmute_one_to_bytes};

#[test]
fn header_serialize() {
    let header = GvdbHeader::new(false, 123, GvdbPointer::NULL);
    assert_eq!(header.is_byteswap().unwrap(), false);
    let data = transmute_one_to_bytes(&header);
    let parsed_header: GvdbHeader = transmute_one_pedantic(data.as_ref()).unwrap();
    assert_eq!(parsed_header.is_byteswap().unwrap(), false);

    let header = GvdbHeader::new(true, 0, GvdbPointer::NULL);
    assert_eq!(header.is_byteswap().unwrap(), true);
    let data = transmute_one_to_bytes(&header);
    let parsed_header: GvdbHeader = transmute_one_pedantic(data.as_ref()).unwrap();
    assert_eq!(parsed_header.is_byteswap().unwrap(), true);
}
