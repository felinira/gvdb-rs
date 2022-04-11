use crate::gvdb::builder::{GvdbItem, SimpleHashTable};
use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash_item::{GvdbHashItem, GvdbValue};
use crate::gvdb::pointer::GvdbPointer;
use crate::gvdb::util::djb_hash;
use safe_transmute::{
    transmute_many_pedantic, transmute_one, transmute_one_pedantic, TriviallyTransmutable,
};
use std::borrow::Cow;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use crate::gvdb::root::GvdbRoot;

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct GvdbHashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

unsafe impl TriviallyTransmutable for GvdbHashHeader {}

impl GvdbHashHeader {
    pub fn new(n_bloom_words: u32, n_buckets: u32) -> Self {
        Self {
            n_bloom_words,
            n_buckets,
        }
    }

    pub fn n_bloom_words(&self) -> u32 {
        u32::from_le(self.n_bloom_words) & (1 << 27) - 1
    }

    pub fn bloom_words_len(&self) -> usize {
        self.n_bloom_words() as usize * size_of::<u32>()
    }

    pub fn n_buckets(&self) -> u32 {
        u32::from_le(self.n_buckets)
    }

    pub fn buckets_len(&self) -> usize {
        self.n_buckets() as usize * size_of::<u32>()
    }
}

impl Debug for GvdbHashHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GvdbHashHeader {{ n_bloom_words: {}, n_buckets: {} }}",
            self.n_bloom_words(),
            self.n_buckets()
        )
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct GvdbHashTable<'a> {
    root: &'a GvdbRoot<'a>,
    table_ptr: GvdbPointer,
    header: GvdbHashHeader,
}

impl<'a> GvdbHashTable<'a> {
    pub fn with_simple_hash_table(table: SimpleHashTable<GvdbItem>, root: &'a GvdbRoot) -> Self {
        let header = GvdbHashHeader::new(0, table.n_buckets() as u32);
        let items_len = table.n_items() * size_of::<GvdbHashItem>();
        let size = size_of::<GvdbHashHeader>()
            + header.bloom_words_len()
            + header.buckets_len()
            + items_len;

        let table_ptr = GvdbPointer::new(0, size);

        Self {
            root,
            table_ptr,
            header,
        }
    }

    /// Interpret a chunk of bytes as a HashTable. The table_ptr should point to the hash table.
    /// Data has to be the complete GVDB file, as hash table items are stored somewhere else.
    pub fn for_bytes(data: &'a [u8], root: &'a GvdbRoot, table_ptr: GvdbPointer) -> GvdbResult<Self> {
        let header = Self::hash_header(data, &table_ptr)?;
        let data = Cow::Borrowed(data);

        let this = Self {
            root,
            table_ptr,
            header,
        };

        let table_data = this.table_ptr.dereference(&this.root.data, 4)?;
        let header_len = size_of::<GvdbHashHeader>();
        let bloom_words_len = this.bloom_words_end() - this.bloom_words_offset();
        let hash_buckets_len = this.hash_buckets_end() - this.hash_buckets_offset();
        let hash_items_len = this.hash_items_end() - this.hash_items_offset();

        let required_len = header_len + bloom_words_len + hash_buckets_len + hash_items_len;

        if required_len > table_data.len() {
            Err(GvdbError::DataError(format!(
                "Not enough bytes to fit hash table: Expected at least {} bytes, got {}",
                required_len,
                table_data.len()
            )))
        } else if hash_items_len % size_of::<GvdbHashItem>() != 0 {
            // Wrong data length
            Err(GvdbError::DataError(format!(
                "Remaining size invalid: Expected a multiple of {}, got {}",
                size_of::<GvdbHashItem>(),
                table_data.len()
            )))
        } else {
            Ok(this)
        }
    }

    /// Read the hash table header
    pub fn hash_header(data: &'a [u8], pointer: &GvdbPointer) -> GvdbResult<GvdbHashHeader> {
        let start = pointer.start() as usize;
        let bytes: &[u8] = data
            .get(start..start + size_of::<GvdbHashHeader>())
            .ok_or(GvdbError::DataOffset)?;

        Ok(transmute_one(bytes)?)
    }

    pub fn get_header(&self) -> GvdbHashHeader {
        self.header
    }

    fn get_u32(&self, offset: usize) -> GvdbResult<u32> {
        let bytes = self
            .root
            .data
            .get(offset..offset + size_of::<u32>())
            .ok_or(GvdbError::DataOffset)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn data_offset(&self) -> usize {
        self.table_ptr.start() as usize
    }

    fn bloom_words_offset(&self) -> usize {
        self.data_offset() + size_of::<GvdbHashHeader>()
    }

    fn bloom_words_end(&self) -> usize {
        self.bloom_words_offset() + self.header.bloom_words_len()
    }

    pub fn bloom_words(&self) -> Option<&[u32]> {
        let data_u8 = self
            .root
            .data
            .get(self.bloom_words_offset()..self.bloom_words_end());
        if let Some(data_u8) = data_u8 {
            transmute_many_pedantic(data_u8).ok()
        } else {
            None
        }
    }

    fn get_bloom_word(&self, index: usize) -> GvdbResult<u32> {
        let start = self.bloom_words_offset() + index * size_of::<u32>();
        self.get_u32(start)
    }

    // TODO: Calculate proper bloom shift
    fn bloom_shift(&self) -> usize {
        0
    }

    /// gvdb_table_bloom_filter
    pub fn bloom_filter(&self, hash_value: u32) -> bool {
        if self.header.n_bloom_words() == 0 {
            return true;
        }

        let word = (hash_value / 32) % self.header.n_bloom_words();
        let mut mask = 1 << (hash_value & 31);
        mask |= 1 << ((hash_value >> self.bloom_shift()) & 31);

        // We know this index is < n_bloom_words
        let bloom_word = self.get_bloom_word(word as usize).unwrap();
        bloom_word & mask == mask
    }

    fn hash_buckets_offset(&self) -> usize {
        self.bloom_words_end()
    }

    fn hash_buckets_end(&self) -> usize {
        self.hash_buckets_offset() + self.header.buckets_len()
    }

    fn get_hash(&self, index: usize) -> GvdbResult<u32> {
        let start = self.hash_buckets_offset() + index * size_of::<u32>();
        self.get_u32(start)
    }

    fn hash_items_offset(&self) -> usize {
        self.hash_buckets_end()
    }

    fn n_hash_items(&self) -> usize {
        let len = self.table_ptr.end() as usize - self.hash_items_offset();
        len / size_of::<GvdbHashItem>()
    }

    fn hash_items_end(&self) -> usize {
        self.table_ptr.end() as usize
    }

    /// Get the hash item at hash item index
    fn get_hash_item_for_index(&self, index: usize) -> GvdbResult<GvdbHashItem> {
        let size = size_of::<GvdbHashItem>();
        let start = self.hash_items_offset() + size * index;
        let end = start + size;

        let data = self.root.data.get(start..end).ok_or(GvdbError::DataOffset)?;
        Ok(transmute_one_pedantic(data)?)
    }

    /// Gets a list of keys
    pub fn get_names(&self) -> GvdbResult<Vec<String>> {
        let count = self.n_hash_items();
        let mut names = vec![None; count];

        let mut inserted = 0;
        while inserted < count {
            let last_inserted = inserted;
            for index in 0..count as usize {
                let item = self.get_hash_item_for_index(index)?;
                let parent: usize = item.parent().try_into()?;

                if names[index] == None {
                    // Only process items not already processed
                    if parent == 0xffffffff {
                        // root item
                        let name = self.root.get_key(&item)?;
                        let _ = std::mem::replace(&mut names[index], Some(name));
                        inserted += 1;
                    } else if parent < count && names[parent].is_some() {
                        // We already came across this item
                        let name = self.root.get_key(&item)?;
                        let parent_name = names.get(parent).unwrap().as_ref().unwrap();
                        let full_name = name + parent_name;
                        let _ = std::mem::replace(&mut names[index], Some(full_name));
                        inserted += 1;
                    } else if parent > count {
                        return Err(GvdbError::DataError(format!(
                            "Parent with invalid offset encountered: {}",
                            parent
                        )));
                    }
                }
            }

            if last_inserted == inserted {
                // No insertion took place this round, there must be a parent loop
                // We fail instead of infinitely looping
                return Err(GvdbError::DataError(
                    "Error finding all parent items. The file appears to have a loop".to_string(),
                ));
            }
        }

        let names = names.into_iter().map(|s| s.unwrap()).collect();
        Ok(names)
    }

    fn check_name(&self, item: &GvdbHashItem, key: &str) -> bool {
        let this_key = match self.root.get_key(item) {
            Ok(this_key) => this_key,
            Err(_) => return false,
        };

        if key != this_key {
            return false;
        }

        let parent = item.parent();
        if key.len() == this_key.len() && parent == 0xffffffff {
            return true;
        }

        if parent < self.n_hash_items() as u32 && key.len() > 0 {
            let parent_item = match self.get_hash_item_for_index(parent as usize) {
                Ok(p) => p,
                Err(_) => return false,
            };

            return self.check_name(&parent_item, key);
        }

        false
    }

    pub fn get_hash_item(&self, key: &str) -> GvdbResult<GvdbHashItem> {
        if self.header.n_buckets() == 0 || self.n_hash_items() == 0 {
            return Err(GvdbError::KeyError);
        }

        let hash_value = djb_hash(key);
        if !self.bloom_filter(hash_value) {
            return Err(GvdbError::KeyError);
        }

        let bucket = hash_value % self.header.n_buckets();
        let mut itemno = self.get_hash(bucket as usize)? as usize;

        let lastno = if bucket == self.header.n_buckets() - 1 {
            self.n_hash_items() as usize
        } else {
            min(
                self.get_hash(bucket as usize + 1)?,
                self.n_hash_items() as u32,
            ) as usize
        };

        while itemno < lastno {
            let item = self.get_hash_item_for_index(itemno)?;
            if hash_value == item.hash_value() {
                if self.check_name(&item, key) {
                    return Ok(item);
                }
            }

            itemno += 1;
        }

        Err(GvdbError::KeyError)
    }

    pub fn get_value(&self, key: &str) -> GvdbResult<glib::Variant> {
        self.root.get_value_for_item(self.get_hash_item(key)?)
    }

    pub fn get_hash_table(&self, key: &str) -> GvdbResult<GvdbHashTable> {
        self.root.get_hash_table_for_item(self.get_hash_item(key)?)
    }

    pub fn get(&self, table: &GvdbHashTable, key: &str) -> GvdbResult<GvdbValue> {
        let item = table.get_hash_item(key)?;

        match item.typ() as char {
            'v' => Ok(GvdbValue::Variant(self.root.get_value_for_item(item)?)),
            'H' => Ok(GvdbValue::HashTable(self.root.get_hash_table_for_item(item)?)),
            _ => Err(GvdbError::InvalidData),
        }
    }
}
