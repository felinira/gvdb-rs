use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::pointer::GvdbPointer;
use deku::prelude::*;

// This is just a string, but it is stored in the byteorder of the file
// Default byteorder is little endian, but the format supports big endian as well
// "GVar"
const GVDB_SIGNATURE0: u32 = 1918981703;
// "iant"
const GVDB_SIGNATURE1: u32 = 1953390953;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct GvdbHeader {
    signature: [u32; 2],
    #[deku(endian = "little")]
    version: u32,
    #[deku(endian = "little")]
    options: u32,
    root: GvdbPointer,
}

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

    pub fn from_bytes_checked(bytes: &[u8]) -> GvdbResult<(&[u8], Self)> {
        let ((rest, _bit_offset), this): ((&[u8], usize), Self) =
            DekuContainerRead::from_bytes((bytes, 0))?;
        if !this.header_valid() {
            Err(GvdbError::InvalidData)
        } else {
            Ok((rest, this))
        }
    }

    pub fn root(&self) -> &GvdbPointer {
        &self.root
    }
}
