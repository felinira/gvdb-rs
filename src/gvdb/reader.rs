use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::header::GvdbHeader;

pub struct GvdbReader {
    header: GvdbHeader,
    byteswap: bool,
}

impl GvdbReader {
    pub fn from_bytes_checked(bytes: &[u8]) -> GvdbResult<Self> {
        let (rest, header) = GvdbHeader::from_bytes_checked(bytes)?;
        let byteswap = header.is_byteswap()?;

        let this = Self { header, byteswap };

        if rest.is_empty() {
            Err(GvdbError::TooMuchData)
        } else {
            Ok(this)
        }
    }
}
