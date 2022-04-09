use deku::DekuContainerWrite;
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;

#[test]
fn header_serialize() {
    let header = GvdbHeader::new(false, 123, GvdbPointer::NULL);
    assert_eq!(header.is_byteswap().unwrap(), false);
    let data = header.to_bytes().unwrap();
    let (_rest, parsed_header) = GvdbHeader::from_bytes_checked(data.as_ref()).unwrap();
    assert_eq!(parsed_header.is_byteswap().unwrap(), false);

    let header = GvdbHeader::new(true, 0, GvdbPointer::NULL);
    assert_eq!(header.is_byteswap().unwrap(), true);
    let data = header.to_bytes().unwrap();
    let (_rest, parsed_header) = GvdbHeader::from_bytes_checked(data.as_ref()).unwrap();
    assert_eq!(parsed_header.is_byteswap().unwrap(), true);
}
