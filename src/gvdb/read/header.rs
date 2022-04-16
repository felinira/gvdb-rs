use crate::gvdb::read::error::{GvdbError, GvdbResult};
use crate::gvdb::read::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;

// This is just a string, but it is stored in the byteorder of the file
// Default byteorder is little endian, but the format supports big endian as well
// "GVar"
const GVDB_SIGNATURE0: u32 = 1918981703;
// "iant"
const GVDB_SIGNATURE1: u32 = 1953390953;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GvdbHeader {
    signature: [u32; 2],
    version: u32,
    options: u32,
    root: GvdbPointer,
}

unsafe impl TriviallyTransmutable for GvdbHeader {}

impl GvdbHeader {
    pub fn new(byteswap: bool, version: u32, root: GvdbPointer) -> Self {
        let signature = if !byteswap {
            [GVDB_SIGNATURE0, GVDB_SIGNATURE1]
        } else {
            [GVDB_SIGNATURE0.swap_bytes(), GVDB_SIGNATURE1.swap_bytes()]
        };

        Self {
            signature,
            version: version.to_le(),
            options: 0,
            root,
        }
    }

    pub fn is_byteswap(&self) -> GvdbResult<bool> {
        if self.signature[0] == GVDB_SIGNATURE0 && self.signature[1] == GVDB_SIGNATURE1 {
            Ok(false)
        } else if self.signature[0] == GVDB_SIGNATURE0.swap_bytes()
            && self.signature[1] == GVDB_SIGNATURE1.swap_bytes()
        {
            Ok(true)
        } else {
            Err(GvdbError::InvalidData)
        }
    }

    pub fn header_valid(&self) -> bool {
        self.is_byteswap().is_ok()
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn options(&self) -> u32 {
        self.options
    }

    pub fn root(&self) -> &GvdbPointer {
        &self.root
    }

    pub fn set_root(&mut self, pointer: GvdbPointer) {
        self.root = pointer;
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
}
