use std::mem::size_of;

use crate::read::error::{Error, Result};
use crate::read::pointer::Pointer;
use safe_transmute::{transmute_one_pedantic, TriviallyTransmutable};

// This is just a string, but it is stored in the byteorder of the file
// Default byteorder is little endian, but the format supports big endian as well
// "GVar"
const GVDB_SIGNATURE0: u32 = 1918981703;
// "iant"
const GVDB_SIGNATURE1: u32 = 1953390953;

/// A GVDB file header.
///
/// ```text
/// +-------+--------------+
/// | Bytes | Field        |
/// +-------+--------------+
/// |     8 | signature    |
/// +-------+--------------+
/// |     4 | version      |
/// +-------+--------------+
/// |     4 | options      |
/// +-------+--------------+
/// |     8 | root pointer |
/// +-------+--------------+
/// ```
///
/// ## Signature
///
/// The signature will look like the ASCII string `GVariant` for little endian
/// and `raVGtnai` for big endian files.
///
/// This is what you get when reading two u32, swapping the endianness, and interpreting them as a string.
///
/// ## Version
///
/// Version is always 0.
///
/// ## Options
///
/// There are no known options, this u32 is always 0.
///
/// ## Root pointer
///
/// Points to the root hash table within the file.

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
    /// Try to read the header, determine the endianness and validate that the header is valid.
    ///
    /// Returns [`Error::DataOffset`]` if the header doesn't fit, and [`Error::Data`] if the header
    /// is invalid.
    pub fn try_from_bytes(data: &[u8]) -> Result<Self> {
        let header_data = data
            .as_ref()
            .get(0..size_of::<Header>())
            .ok_or(Error::DataOffset)?;
        let header: Self = transmute_one_pedantic(header_data)?;

        if !header.header_valid() {
            return Err(Error::Data(
                "Invalid GVDB header. Is this a GVDB file?".to_string(),
            ));
        }

        if header.version() != 0 {
            return Err(Error::Data(format!(
                "Unknown GVDB file format version: {}",
                header.version()
            )));
        }

        Ok(header)
    }

    /// Create a new GVDB header in little-endian
    #[cfg(test)]
    pub fn new_le(version: u32, root: Pointer) -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = false;
        #[cfg(target_endian = "big")]
        let byteswap = true;

        Self::new(byteswap, version, root)
    }

    /// Create a new GVDB header in big-endian
    #[cfg(test)]
    pub fn new_be(version: u32, root: Pointer) -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = true;
        #[cfg(target_endian = "big")]
        let byteswap = false;

        Self::new(byteswap, version, root)
    }

    /// Create a new GVDB header in target endianness
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

    /// Returns:
    ///
    /// - `Ok(true)` if the file is *not* in target endianness (eg. BE on an LE machine)
    /// - `Ok(false)` if the file is in target endianness (eg. LE on an LE machine)
    /// - [`Err(Error::Data)`](crate::read::error::Error::Data) if the file signature is invalid
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

    /// Returns true if the header indicates that this is a valid GVDB file.
    pub fn header_valid(&self) -> bool {
        self.is_byteswap().is_ok()
    }

    /// The version of the GVDB file. We only recognize version 0 of the format.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// The pointer to the root hash table.
    pub fn root(&self) -> &Pointer {
        &self.root
    }

    pub fn dereference<'a>(
        &self,
        data: &'a [u8],
        pointer: &Pointer,
        alignment: u32,
    ) -> Result<&'a [u8]> {
        let start: usize = pointer.start() as usize;
        let end: usize = pointer.end() as usize;
        let alignment: usize = alignment as usize;

        if start > end {
            Err(Error::DataOffset)
        } else if start & (alignment - 1) != 0 {
            Err(Error::DataAlignment)
        } else {
            data.get(start..end).ok_or(Error::DataOffset)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use safe_transmute::{transmute_one_pedantic, transmute_one_to_bytes};

    #[test]
    fn derives() {
        let header = Header::new(false, 0, Pointer::NULL);
        let header2 = header;
        println!("{:?}", header2);
    }

    #[test]
    fn header_serialize() {
        let header = Header::new(false, 123, Pointer::NULL);
        assert!(!header.is_byteswap().unwrap());
        let data = transmute_one_to_bytes(&header);
        let parsed_header: Header = transmute_one_pedantic(data).unwrap();
        assert!(!parsed_header.is_byteswap().unwrap());

        let header = Header::new(true, 0, Pointer::NULL);
        assert!(header.is_byteswap().unwrap());
        let data = transmute_one_to_bytes(&header);
        let parsed_header: Header = transmute_one_pedantic(data).unwrap();
        assert!(parsed_header.is_byteswap().unwrap());
    }
}
