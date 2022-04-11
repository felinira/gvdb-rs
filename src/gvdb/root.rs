use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash::GvdbHashTable;
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
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
    /// gvdb_table_dereference
    fn deref_pointer(&self, pointer: &GvdbPointer, alignment: u32) -> GvdbResult<&[u8]> {
        let start: usize = pointer.start() as usize;
        let end: usize = pointer.end() as usize;
        let alignment: usize = alignment.try_into()?;

        if start > end {
            Err(GvdbError::DataOffset)
        } else if start & (alignment - 1) != 0 {
            Err(GvdbError::DataAlignment)
        } else {
            self.data.get(start..end).ok_or(GvdbError::DataOffset)
        }
    }

    fn get_header(&self) -> GvdbResult<GvdbHeader> {
        let header_data = self
            .data
            .get(0..size_of::<GvdbHeader>())
            .ok_or(GvdbError::DataOffset)?;
        Ok(transmute_one_pedantic(header_data)?)
    }

    pub fn get_hash_table_root(&self) -> GvdbResult<GvdbHashTable> {
        let header = self.get_header()?;
        let root_ptr = header.root().clone();
        Ok(GvdbHashTable::for_bytes(&self.data, root_ptr)?)
    }

    /// gvdb_table_new_from_bytes
    pub fn from_bytes(bytes: &[u8], trusted: bool) -> GvdbResult<Self> {
        let mut this = Self {
            data: bytes.to_vec(),
            byteswapped: false,
            trusted,
        };

        let header = this.get_header()?;
        println!("{:?}", header);
        this.byteswapped = header.is_byteswap()?;
        //this.setup_root(header.root())?;
        Ok(this)
    }

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
