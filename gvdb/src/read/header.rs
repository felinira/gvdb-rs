use crate::read::error::{Error, Result};
use crate::read::pointer::Pointer;
use safe_transmute::TriviallyTransmutable;

// This is just a string, but it is stored in the byteorder of the file
// Default byteorder is little endian, but the format supports big endian as well
// "GVar"
const GVDB_SIGNATURE0: u32 = 1918981703;
// "iant"
const GVDB_SIGNATURE1: u32 = 1953390953;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Header {
    signature: [u32; 2],
    version: u32,
    options: u32,
    root: Pointer,
}

unsafe impl TriviallyTransmutable for Header {}

impl Header {
    #[cfg(test)]
    pub fn new_le(version: u32, root: Pointer) -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = false;
        #[cfg(target_endian = "big")]
        let byteswap = true;

        Self::new(byteswap, version, root)
    }

    #[cfg(test)]
    pub fn new_be(version: u32, root: Pointer) -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = true;
        #[cfg(target_endian = "big")]
        let byteswap = false;

        Self::new(byteswap, version, root)
    }

    pub fn new(byteswap: bool, version: u32, root: Pointer) -> Self {
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

    pub fn is_byteswap(&self) -> Result<bool> {
        if self.signature[0] == GVDB_SIGNATURE0 && self.signature[1] == GVDB_SIGNATURE1 {
            Ok(false)
        } else if self.signature[0] == GVDB_SIGNATURE0.swap_bytes()
            && self.signature[1] == GVDB_SIGNATURE1.swap_bytes()
        {
            Ok(true)
        } else {
            Err(Error::Data(format!(
                "Invalid GVDB header signature: {:?}. Is this a GVariant database file?",
                self.signature
            )))
        }
    }

    pub fn header_valid(&self) -> bool {
        self.is_byteswap().is_ok()
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn root(&self) -> &Pointer {
        &self.root
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use safe_transmute::{transmute_one_pedantic, transmute_one_to_bytes};

    #[test]
    fn derives() {
        let header = Header::new(false, 0, Pointer::NULL);
        let header2 = header.clone();
        println!("{:?}", header2);
    }

    #[test]
    fn header_serialize() {
        let header = Header::new(false, 123, Pointer::NULL);
        assert_eq!(header.is_byteswap().unwrap(), false);
        let data = transmute_one_to_bytes(&header);
        let parsed_header: Header = transmute_one_pedantic(data.as_ref()).unwrap();
        assert_eq!(parsed_header.is_byteswap().unwrap(), false);

        let header = Header::new(true, 0, Pointer::NULL);
        assert_eq!(header.is_byteswap().unwrap(), true);
        let data = transmute_one_to_bytes(&header);
        let parsed_header: Header = transmute_one_pedantic(data.as_ref()).unwrap();
        assert_eq!(parsed_header.is_byteswap().unwrap(), true);
    }
}
