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
pub struct GvdbTable {
    data: Vec<u8>,

    byteswapped: bool,
    trusted: bool,

    bloom_words_offset: usize,
    n_bloom_words: u32,
    bloom_shift: usize,

    hash_buckets_offset: usize,
    n_buckets: u32,

    hash_items_offset: usize,
    n_hash_items: u32,
}

impl GvdbTable {
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

    /// gvdb_table_setup_root
    /*pub fn setup_root(&mut self, pointer: &GvdbPointer) -> GvdbResult<()> {
        let mut size: usize = pointer.size().try_into()?;
        let root_bytes = self.deref_pointer(pointer, 4)?;

        let header: GvdbHashHeader = transmute_one(root_bytes)?;
        size -= size_of::<GvdbHashHeader>();
        println!("{:?}", header);

        self.bloom_words_offset = pointer.start() as usize + size_of::<GvdbHashHeader>();
        self.n_bloom_words = header.n_bloom_words();
        size -= self.n_bloom_words as usize * size_of::<u32>();

        self.hash_buckets_offset =
            self.bloom_words_offset + self.n_bloom_words as usize * size_of::<u32>();
        self.n_buckets = header.n_buckets();
        size -= self.n_buckets as usize * size_of::<u32>();

        self.hash_items_offset =
            self.hash_buckets_offset + self.n_buckets as usize * size_of::<u32>();
        self.n_hash_items = (size / size_of::<GvdbHashItem>()) as u32;
        if size % size_of::<GvdbHashItem>() != 0 {
            return Err(GvdbError::DataError(format!(
                "Remaining size invalid: Expected a multiple of {}, got {}",
                size_of::<GvdbHashItem>(),
                size
            )));
        }

        println!("{:?}", self);

        Ok(())
    }*/

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
            bloom_words_offset: 0,
            n_bloom_words: 0,
            bloom_shift: 0,
            hash_buckets_offset: 0,
            n_buckets: 0,
            hash_items_offset: 0,
            n_hash_items: 0,
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
            bloom_words_offset: 0,
            n_bloom_words: 0,
            bloom_shift: 0,
            hash_buckets_offset: 0,
            n_buckets: 0,
            hash_items_offset: 0,
            n_hash_items: 0,
        }
    }
}

impl Default for GvdbTable {
    fn default() -> Self {
        Self::empty(true)
    }
}
