use crate::read::error::{GvdbReaderError, GvdbReaderResult};
use crate::read::file::GvdbFile;
use crate::read::hash_item::GvdbHashItem;
use crate::util::djb_hash;
use safe_transmute::{
    transmute_many_pedantic, transmute_one, transmute_one_pedantic, TriviallyTransmutable,
};
use std::borrow::Cow;
use std::cmp::{max, min};
use std::fmt::{Debug, Formatter};
use std::mem::size_of;

/// The header of a GVDB hash table
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct GvdbHashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

unsafe impl TriviallyTransmutable for GvdbHashHeader {}

impl GvdbHashHeader {
    /// Create a new GvdbHashHeader using the provided `bloom_shift`, `n_bloom_words` and
    /// `n_buckets`
    pub fn new(bloom_shift: u32, n_bloom_words: u32, n_buckets: u32) -> Self {
        assert!(n_bloom_words < (1 << 27));
        let n_bloom_words = bloom_shift << 27 | n_bloom_words;

        Self {
            n_bloom_words: n_bloom_words.to_le(),
            n_buckets: n_buckets.to_le(),
        }
    }

    /// Number of bloom words in the hash table header
    pub fn n_bloom_words(&self) -> u32 {
        u32::from_le(self.n_bloom_words) & ((1 << 27) - 1)
    }

    /// Size of the bloom words section in the header
    pub fn bloom_words_len(&self) -> usize {
        self.n_bloom_words() as usize * size_of::<u32>()
    }

    /// Number of hash buckets in the hash table header
    pub fn n_buckets(&self) -> u32 {
        u32::from_le(self.n_buckets)
    }

    /// Length of the hash buckets section in the header
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

/// A hash table inside a GVDB file
///
///
#[repr(C)]
#[derive(Clone, Debug)]
pub struct GvdbHashTable<'a> {
    pub(crate) root: &'a GvdbFile,
    data: Cow<'a, [u8]>,
    header: GvdbHashHeader,
}

impl<'a> GvdbHashTable<'a> {
    /// Interpret a chunk of bytes as a HashTable. The table_ptr should point to the hash table.
    /// Data has to be the complete GVDB file, as hash table items are stored somewhere else.
    pub fn for_bytes(data: &'a [u8], root: &'a GvdbFile) -> GvdbReaderResult<Self> {
        let header = Self::hash_header(data)?;
        let data = Cow::Borrowed(data);

        let this = Self { root, data, header };

        let header_len = size_of::<GvdbHashHeader>();
        let bloom_words_len = this.bloom_words_end() - this.bloom_words_offset();
        let hash_buckets_len = this.hash_buckets_end() - this.hash_buckets_offset();

        // we use max() here to prevent possible underflow
        let hash_items_len =
            max(this.hash_items_end(), this.hash_items_offset()) - this.hash_items_offset();
        let required_len = header_len + bloom_words_len + hash_buckets_len + hash_items_len;

        if required_len > this.data.len() {
            Err(GvdbReaderError::DataError(format!(
                "Not enough bytes to fit hash table: Expected at least {} bytes, got {}",
                required_len,
                this.data.len()
            )))
        } else if hash_items_len % size_of::<GvdbHashItem>() != 0 {
            // Wrong data length
            Err(GvdbReaderError::DataError(format!(
                "Remaining size invalid: Expected a multiple of {}, got {}",
                size_of::<GvdbHashItem>(),
                this.data.len()
            )))
        } else {
            Ok(this)
        }
    }

    /// Read the hash table header
    fn hash_header(data: &'a [u8]) -> GvdbReaderResult<GvdbHashHeader> {
        let bytes: &[u8] = data
            .get(0..size_of::<GvdbHashHeader>())
            .ok_or(GvdbReaderError::DataOffset)?;

        Ok(transmute_one(bytes)?)
    }

    /// Returns the header for this hash table
    pub fn get_header(&self) -> GvdbHashHeader {
        self.header
    }

    fn get_u32(&self, offset: usize) -> GvdbReaderResult<u32> {
        let bytes = self
            .data
            .get(offset..offset + size_of::<u32>())
            .ok_or(GvdbReaderError::DataOffset)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn bloom_words_offset(&self) -> usize {
        size_of::<GvdbHashHeader>()
    }

    fn bloom_words_end(&self) -> usize {
        self.bloom_words_offset() + self.header.bloom_words_len()
    }

    /// Returns the bloom words for this hash table
    #[allow(dead_code)]
    fn bloom_words(&self) -> Option<&[u32]> {
        // This indexing operation is safe as data is guaranteed to be larger than
        // bloom_words_offset and this will just return an empty slice if end == offset
        transmute_many_pedantic(&self.data[self.bloom_words_offset()..self.bloom_words_end()]).ok()
    }

    fn get_bloom_word(&self, index: usize) -> GvdbReaderResult<u32> {
        if index >= self.header.n_bloom_words() as usize {
            return Err(GvdbReaderError::DataOffset);
        }

        let start = self.bloom_words_offset() + index * size_of::<u32>();
        self.get_u32(start)
    }

    // TODO: Calculate proper bloom shift
    fn bloom_shift(&self) -> usize {
        0
    }

    /// Check whether the hash value corresponds to the bloom filter
    fn bloom_filter(&self, hash_value: u32) -> bool {
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

    fn get_hash(&self, index: usize) -> GvdbReaderResult<u32> {
        let start = self.hash_buckets_offset() + index * size_of::<u32>();
        self.get_u32(start)
    }

    fn hash_items_offset(&self) -> usize {
        self.hash_buckets_end()
    }

    fn n_hash_items(&self) -> usize {
        let len = self.hash_items_end() - self.hash_items_offset();
        len / size_of::<GvdbHashItem>()
    }

    fn hash_items_end(&self) -> usize {
        self.data.len()
    }

    /// Get the hash item at hash item index
    fn get_hash_item_for_index(&self, index: usize) -> GvdbReaderResult<GvdbHashItem> {
        let size = size_of::<GvdbHashItem>();
        let start = self.hash_items_offset() + size * index;
        let end = start + size;

        let data = self
            .data
            .get(start..end)
            .ok_or(GvdbReaderError::DataOffset)?;
        Ok(transmute_one_pedantic(data)?)
    }

    /// Gets a list of keys contained in the hash table
    pub fn get_names(&self) -> GvdbReaderResult<Vec<String>> {
        let count = self.n_hash_items();
        let mut names = vec![None; count];

        let mut inserted = 0;
        while inserted < count {
            let last_inserted = inserted;
            for index in 0..count {
                let item = self.get_hash_item_for_index(index)?;
                let parent: usize = item.parent().try_into()?;

                if names[index].is_none() {
                    // Only process items not already processed
                    if parent == 0xffffffff {
                        // root item
                        let name = self.get_key(&item)?;
                        let _ = std::mem::replace(&mut names[index], Some(name));
                        inserted += 1;
                    } else if parent < count && names[parent].is_some() {
                        // We already came across this item
                        let name = self.get_key(&item)?;
                        let parent_name = names.get(parent).unwrap().as_ref().unwrap();
                        let full_name = parent_name.to_string() + &name;
                        let _ = std::mem::replace(&mut names[index], Some(full_name));
                        inserted += 1;
                    } else if parent > count {
                        return Err(GvdbReaderError::DataError(format!(
                            "Parent with invalid offset encountered: {}",
                            parent
                        )));
                    }
                }
            }

            if last_inserted == inserted {
                // No insertion took place this round, there must be a parent loop
                // We fail instead of infinitely looping
                return Err(GvdbReaderError::DataError(
                    "Error finding all parent items. The file appears to have a loop".to_string(),
                ));
            }
        }

        let names = names.into_iter().map(|s| s.unwrap()).collect();
        Ok(names)
    }

    fn check_name(&self, item: &GvdbHashItem, key: &str) -> bool {
        let this_key = match self.get_key(item) {
            Ok(this_key) => this_key,
            Err(_) => return false,
        };

        if !key.ends_with(&this_key) {
            return false;
        }

        let parent = item.parent();
        if key.len() == this_key.len() && parent == 0xffffffff {
            return true;
        }

        if parent < self.n_hash_items() as u32 && !key.is_empty() {
            let parent_item = match self.get_hash_item_for_index(parent as usize) {
                Ok(p) => p,
                Err(_) => return false,
            };

            let parent_key_len = key.len() - this_key.len();
            return self.check_name(&parent_item, &key[0..parent_key_len]);
        }

        false
    }

    /// Gets the item at key `key`
    pub fn get_hash_item(&self, key: &str) -> GvdbReaderResult<GvdbHashItem> {
        if self.header.n_buckets() == 0 || self.n_hash_items() == 0 {
            return Err(GvdbReaderError::KeyError(key.to_string()));
        }

        let hash_value = djb_hash(key);
        if !self.bloom_filter(hash_value) {
            return Err(GvdbReaderError::KeyError(key.to_string()));
        }

        let bucket = hash_value % self.header.n_buckets();
        let mut itemno = self.get_hash(bucket as usize)? as usize;

        let lastno = if bucket == self.header.n_buckets() - 1 {
            self.n_hash_items()
        } else {
            min(
                self.get_hash(bucket as usize + 1)?,
                self.n_hash_items() as u32,
            ) as usize
        };

        while itemno < lastno {
            let item = self.get_hash_item_for_index(itemno)?;
            if hash_value == item.hash_value() && self.check_name(&item, key) {
                return Ok(item);
            }

            itemno += 1;
        }

        Err(GvdbReaderError::KeyError(key.to_string()))
    }

    /// Get the item at key `key` and try to interpret it as a [`enum@zvariant::Value`]
    pub fn get_value(&self, key: &str) -> GvdbReaderResult<zvariant::Value> {
        self.get_value_for_item(&self.get_hash_item(key)?)
    }

    /// Get the item at key `key` and try to convert it from [`enum@zvariant::Value`] to T
    pub fn get<T: ?Sized>(&self, key: &str) -> GvdbReaderResult<T>
    where
        T: TryFrom<zvariant::OwnedValue>,
    {
        T::try_from(zvariant::OwnedValue::from(
            self.get_value_for_item(&self.get_hash_item(key)?)?,
        ))
        .map_err(|_| {
            GvdbReaderError::DataError("Can't convert Value to specified type".to_string())
        })
    }

    #[cfg(feature = "glib")]
    /// Get the item at key `key` and try to interpret it as a [`struct@glib::Variant`]
    pub fn get_gvariant(&self, key: &str) -> GvdbReaderResult<glib::Variant> {
        self.get_gvariant_for_item(&self.get_hash_item(key)?)
    }

    /// Get the item at key `key` and try to interpret it as a [`GvdbHashTable`]
    pub fn get_hash_table(&self, key: &str) -> GvdbReaderResult<GvdbHashTable> {
        self.get_hash_table_for_item(&self.get_hash_item(key)?)
    }

    fn get_key(&self, item: &GvdbHashItem) -> GvdbReaderResult<String> {
        self.root.get_key(item)
    }

    fn get_value_for_item(&self, item: &GvdbHashItem) -> GvdbReaderResult<zvariant::Value> {
        self.root.get_value_for_item(item)
    }

    #[cfg(feature = "glib")]
    fn get_gvariant_for_item(&self, item: &GvdbHashItem) -> GvdbReaderResult<glib::Variant> {
        self.root.get_gvariant_for_item(item)
    }

    fn get_hash_table_for_item(&self, item: &GvdbHashItem) -> GvdbReaderResult<GvdbHashTable> {
        self.root.get_hash_table_for_item(item)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::read::{GvdbFile, GvdbHashHeader, GvdbPointer, GvdbReaderError};
    use crate::test::*;
    use crate::test::{assert_eq, assert_matches, assert_ne};

    #[test]
    fn derives() {
        let header = GvdbHashHeader::new(0, 0, 0);
        let header2 = header.clone();
        println!("{:?}", header2);

        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let table2 = table.clone();
        println!("{:?}", table2);
    }

    #[test]
    fn get_header() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let header = table.get_header();
        assert_eq!(header.n_buckets(), 0);

        let file = new_simple_file(false);
        let table = file.hash_table().unwrap();
        let header = table.get_header();
        assert_eq!(header.n_buckets(), 1);
    }

    #[test]
    fn bloom_words() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let header = table.get_header();
        assert_eq!(header.n_bloom_words(), 0);
        assert_eq!(header.bloom_words_len(), 0);
        assert_eq!(table.bloom_words(), None);
    }

    #[test]
    fn get_item() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let res = table.get_hash_item("test");
        assert_matches!(res, Err(GvdbReaderError::KeyError(_)));

        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let item = table.get_hash_item("test").unwrap();
            assert_ne!(item.value_ptr(), &GvdbPointer::NULL);
            let value: String = table.get_value_for_item(&item).unwrap().try_into().unwrap();
            assert_eq!(value, "test");

            let item_fail = table.get_hash_item("fail").unwrap_err();
            assert_matches!(item_fail, GvdbReaderError::KeyError(_));

            let res_item = table.get_hash_item("test_fail");
            assert_matches!(res_item, Err(GvdbReaderError::KeyError(_)));
        }
    }

    #[test]
    fn get() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res: String = table.get::<String>("test").unwrap().into();
            assert_eq!(&res, "test");

            let res = table.get::<i32>("test");
            assert_matches!(res, Err(GvdbReaderError::DataError(_)));
        }
    }

    #[test]
    fn get_bloom_word() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res = table.get_bloom_word(0);
            assert_matches!(res, Err(GvdbReaderError::DataOffset));
        }
    }

    #[test]
    fn bloom_shift() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res = table.bloom_shift();
            assert_eq!(res, 0);
        }
    }

    #[test]
    fn get_value() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res = table.get_value("test").unwrap();
            assert_eq!(&res, &zvariant::Value::from("test"));

            let fail = table.get_value("fail").unwrap_err();
            assert_matches!(fail, GvdbReaderError::KeyError(_));
        }
    }

    #[test]
    fn get_hash_table() {
        let file = GvdbFile::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let table = table.get_hash_table("table").unwrap();
        let fail = table.get_hash_table("fail").unwrap_err();
        assert_matches!(fail, GvdbReaderError::KeyError(_));
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::GvdbReaderError;
    use crate::test::new_simple_file;
    use glib::ToVariant;
    use matches::assert_matches;

    #[test]
    fn get_gvariant() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res: glib::Variant = table.get_gvariant("test").unwrap().get().unwrap();
            assert_eq!(&res, &"test".to_variant());

            let fail = table.get_gvariant("fail").unwrap_err();
            assert_matches!(fail, GvdbReaderError::KeyError(_));
        }
    }
}
