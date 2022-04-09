use std::cmp::min;
use std::fs::File;
use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash::{GvdbHashHeader, GvdbHashItem};
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use crate::gvdb::util::{djb_hash, ReadFromBytes};
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
    /// gvdb_table_item_get_key
    pub fn get_key(&self, item: &GvdbHashItem) -> GvdbResult<String> {
        let start: usize = item.key_start().try_into()?;
        let size: usize = item.key_size().into();
        let end = start + size;

        let data = self.data.get(start..end).ok_or(GvdbError::DataOffset)?;
        Ok(String::from_utf8(data.to_vec())?)
    }

    /// gvdb_table_dereference
    pub fn deref_pointer(&self, pointer: &GvdbPointer, alignment: u32) -> GvdbResult<&[u8]> {
        let start: usize = pointer.start().try_into()?;
        let end: usize = pointer.end().try_into()?;
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
    pub fn setup_root(&mut self, pointer: &GvdbPointer) -> GvdbResult<()> {
        let mut size: usize = pointer.size().try_into()?;
        let hash_header_bytes = self.deref_pointer(pointer, 4)?;
        let (_rest, header) = GvdbHashHeader::from_bytes_aligned(hash_header_bytes, 1)?;
        size -= size_of::<GvdbHashHeader>();
        println!("{:?}", header);

        self.bloom_words_offset = pointer.start() as usize + size_of::<GvdbHashHeader>();
        self.n_bloom_words = header.n_bloom_words();
        size -= self.n_bloom_words as usize * size_of::<u32>();

        self.hash_buckets_offset = self.bloom_words_offset + self.n_bloom_words as usize * size_of::<u32>();
        self.n_buckets = header.n_buckets();
        size -= self.n_buckets as usize * size_of::<u32>();

        self.hash_items_offset = self.hash_buckets_offset + self.n_buckets as usize * size_of::<u32>();
        self.n_hash_items = (size / size_of::<GvdbHashItem>()) as u32;
        if size % size_of::<GvdbHashItem>() != 0 {
            return Err(GvdbError::DataError(format!("Remaining size invalid: Expected a multiple of {}, got {}", size_of::<GvdbHashItem>(), size)));
        }

        println!("{:?}", self);

        Ok(())
    }

    fn get_u32(&self, offset: usize) -> Option<u32> {
        let bytes = self.data.get(offset..offset + size_of::<u32>())?;
        Some(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn get_hash(&self, index: usize) -> Option<u32> {
        let start = self.hash_buckets_offset + index * size_of::<u32>();
        self.get_u32(start)
    }

    fn get_bloom_word(&self, index: usize) -> Option<u32> {
        let start = self.bloom_words_offset + index * size_of::<u32>();
        self.get_u32(start)
    }

    /// gvdb_table_bloom_filter
    pub fn bloom_filter(&self, hash_value: u32) -> bool {
        if self.n_bloom_words == 0 {
            return true;
        }

        let word = (hash_value / 32) % self.n_bloom_words;
        let mut mask = 1 << (hash_value & 31);
        mask |= 1 << ((hash_value >> self.bloom_shift) & 31);

        // We know this index is < n_bloom_words
        let bloom_word = self.get_bloom_word(word as usize).unwrap();
        bloom_word & mask == mask
    }

    /// Get the hash item at hash item index
    fn get_hash_item(&self, index: usize) -> GvdbResult<GvdbHashItem> {
        let size = size_of::<GvdbHashItem>();
        let start = self.hash_items_offset + size * index;
        let end = start + size;

        let data = self.data.get(start..end).ok_or(GvdbError::DataOffset)?;
        GvdbHashItem::from_bytes_aligned_exact(data, 1)
    }

    /// Gets a list of keys
    pub fn get_names(&self) -> GvdbResult<Vec<String>> {
        let count = self.n_hash_items.try_into()?;
        let mut names = vec![None; count];

        let mut inserted = 0;
        while inserted < count {
            let last_inserted = inserted;
            for index in 0..self.n_hash_items as usize {
                let item = self.get_hash_item(index)?;
                let parent: usize = item.parent().try_into()?;
                if parent == 0xffffffff {
                    // root item
                    let name = self.get_key(&item)?;
                    let _ = std::mem::replace(&mut names[index], Some(name));
                    inserted += 1;
                } else if parent < count && names[parent].is_some() {
                    // We already came across this item
                    let name = self.get_key(&item)?;
                    let parent_name = names.get(parent).unwrap().as_ref().unwrap();
                    let full_name = name + parent_name;
                    let _ = std::mem::replace(&mut names[index], Some(full_name));
                    inserted += 1;
                } else if parent > count {
                    return Err(GvdbError::DataError(format!("Parent with invalid offset encountered: {}", parent)));
                }
            }

            if last_inserted == inserted {
                // No insertion took place this round, there must be a parent loop
                // We fail instead of infinitely looping
                return Err(GvdbError::InvalidData);
            }
        }

        let names = names.into_iter().map(|s| s.unwrap()).collect();
        Ok(names)
    }

    fn check_name(&self, item: &GvdbHashItem, key: &str) -> bool {
        let this_key = match self.get_key(item) {
            Ok(this_key) => this_key,
            Err(_) => return false
        };

        if key != this_key {
            return false;
        }

        let parent = item.parent();
        if key.len() == this_key.len() && parent == 0xffffffff {
            return true;
        }

        if parent < self.n_hash_items && key.len() > 0 {
            let parent_item = match self.get_hash_item(parent as usize) {
                Ok(p) => p,
                Err(_) => return false
            };

            return self.check_name(&parent_item, key);
        }

        false
    }

    fn table_lookup(&self, key: &str, typ: char) -> Option<GvdbHashItem> {
        if self.n_buckets == 0 || self.n_hash_items == 0 {
            return None;
        }

        let hash_value = djb_hash(key);
        if !self.bloom_filter(hash_value) {
            return None
        }

        let bucket = hash_value % self.n_buckets;
        let mut itemno = self.get_hash(bucket as usize)? as usize;

        let lastno = if bucket == self.n_buckets - 1 {
            self.n_hash_items as usize
        } else {
            min(self.get_hash(bucket as usize + 1)?, self.n_hash_items) as usize
        };

        while itemno < lastno {
            let item = self.get_hash_item(itemno).ok()?;
            if hash_value == item.hash_value() {
                if self.check_name(&item, key) {
                    if item.typ() == typ {
                        return Some(item);
                    }
                }
            }

            itemno += 1;
        }

        None
    }

    pub fn value_from_item(&self, item: &GvdbHashItem) -> Option<glib::Variant> {
        let data: &[u8] = self.deref_pointer(&item.value_ptr(), 8).ok()?;
        Some(glib::Variant::from_data_with_type(data, glib::VariantTy::VARIANT))
    }

    pub fn get_value(&self, key: &str) -> Option<glib::Variant> {
        let item = self.table_lookup(key, 'v')?;
        self.value_from_item(&item)
    }

    /// gvdb_table_new_from_bytes
    pub fn from_bytes(bytes: &[u8], trusted: bool) -> GvdbResult<Self> {
        let (_rest, header) = GvdbHeader::from_bytes_aligned(bytes, 1)?;
        println!("{:?}", header);
        let byteswapped = header.is_byteswap()?;

        let mut this = Self {
            data: bytes.to_vec(),
            byteswapped,
            trusted,
            bloom_words_offset: 0,
            n_bloom_words: 0,
            bloom_shift: 0,
            hash_buckets_offset: 0,
            n_buckets: 0,
            hash_items_offset: 0,
            n_hash_items: 0
        };

        this.setup_root(header.root())?;
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
