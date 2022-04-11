use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash::GvdbHashTable;
use crate::gvdb::header::GvdbHeader;
use safe_transmute::transmute_one_pedantic;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Debug)]
pub struct GvdbRoot {
    data: Vec<u8>,

    byteswapped: bool,
    trusted: bool,
}

impl GvdbRoot {
    /// Get the GVDB file header. Will err with GvdbError::DataOffset if the header doesn't fit
    fn get_header(&self) -> GvdbResult<GvdbHeader> {
        let header_data = self
            .data
            .get(0..size_of::<GvdbHeader>())
            .ok_or(GvdbError::DataOffset)?;
        Ok(transmute_one_pedantic(header_data)?)
    }

    /// Returns the root hash table of the file
    pub fn hash_table(&self) -> GvdbResult<GvdbHashTable> {
        let header = self.get_header()?;
        let root_ptr = header.root().clone();
        Ok(GvdbHashTable::for_bytes(&self.data, root_ptr)?)
    }

    /// Interpret a chunk of bytes as a GVDB file
    pub fn from_bytes(bytes: &[u8], trusted: bool) -> GvdbResult<Self> {
        let mut this = Self {
            data: bytes.to_vec(),
            byteswapped: false,
            trusted,
        };

        let header = this.get_header()?;
        this.byteswapped = header.is_byteswap()?;
        //this.setup_root(header.root())?;
        Ok(this)
    }

    /// Open a file and interpret the data as GVDB
    pub fn from_file(filename: &Path) -> GvdbResult<Self> {
        let mut file = File::open(filename)?;
        let mut data = Vec::with_capacity(file.metadata()?.len() as usize);
        file.read_to_end(&mut data)?;
        Self::from_bytes(&data, false)
    }

    /// gvdb_table_new
    pub fn empty(trusted: bool) -> Self {
        Self {
            data: vec![],
            byteswapped: false,
            trusted,
        }
    }
}

impl Default for GvdbRoot {
    fn default() -> Self {
        Self::empty(true)
    }
}
