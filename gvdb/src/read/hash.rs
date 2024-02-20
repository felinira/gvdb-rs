use crate::read::error::{Error, Result};
use crate::read::file::File;
use crate::read::hash_item::HashItem;
use crate::util::djb_hash;
use safe_transmute::{
    transmute_many_pedantic, transmute_one, transmute_one_pedantic, TriviallyTransmutable,
};
use serde::Deserialize;
use std::cmp::{max, min};
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use zvariant::Type;

use super::{HashItemType, Pointer};

#[cfg(unix)]
type GVariantDeserializer<'de, 'sig, 'f> =
    zvariant::gvariant::Deserializer<'de, 'sig, 'f, zvariant::Fd<'f>>;
#[cfg(not(unix))]
type GVariantDeserializer<'de, 'sig, 'f> = zvariant::gvariant::Deserializer<'de, 'sig, 'f, ()>;

/// The header of a GVDB hash table
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct HashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

unsafe impl TriviallyTransmutable for HashHeader {}

impl HashHeader {
    /// Create a new [`HashHeader`]` using the provided `bloom_shift`, `n_bloom_words` and
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

impl Debug for HashHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashHeader")
            .field("n_bloom_words", &self.n_bloom_words())
            .field("n_buckets", &self.n_buckets())
            .field("data", &safe_transmute::transmute_one_to_bytes(self))
            .finish()
    }
}

/// A hash table inside a GVDB file
///
///
#[derive(Clone)]
pub struct HashTable<'a, 'file> {
    pub(crate) file: &'a File<'file>,
    pointer: Pointer,
    header: HashHeader,
}

impl<'a, 'file> HashTable<'a, 'file> {
    /// Interpret a chunk of bytes as a HashTable. The table_ptr should point to the hash table.
    /// Data has to be the complete GVDB file, as hash table items are stored somewhere else.
    pub fn for_bytes(pointer: Pointer, root: &'a File<'file>) -> Result<Self> {
        let data = root.dereference(&pointer, 4)?;
        let header = Self::hash_header(data)?;

        let this = Self {
            file: root,
            pointer,
            header,
        };

        let header_len = size_of::<HashHeader>();
        let bloom_words_len = this.bloom_words_end() - this.bloom_words_offset();
        let hash_buckets_len = this.hash_buckets_end() - this.hash_buckets_offset();

        // we use max() here to prevent possible underflow
        let hash_items_len =
            max(this.hash_items_end(), this.hash_items_offset()) - this.hash_items_offset();
        let required_len = header_len + bloom_words_len + hash_buckets_len + hash_items_len;

        if required_len > data.len() {
            Err(Error::Data(format!(
                "Not enough bytes to fit hash table: Expected at least {} bytes, got {}",
                required_len,
                data.len()
            )))
        } else if hash_items_len % size_of::<HashItem>() != 0 {
            // Wrong data length
            Err(Error::Data(format!(
                "Remaining size invalid: Expected a multiple of {}, got {}",
                size_of::<HashItem>(),
                data.len()
            )))
        } else {
            Ok(this)
        }
    }

    /// Read the hash table header
    fn hash_header(data: &'a [u8]) -> Result<HashHeader> {
        let bytes: &[u8] = data
            .get(0..size_of::<HashHeader>())
            .ok_or(Error::DataOffset)?;

        Ok(transmute_one(bytes)?)
    }

    /// A reference to the data section of this [`HashTable`]
    fn data(&self) -> Result<&[u8]> {
        self.file.dereference(&self.pointer, 4)
    }

    /// Returns the header for this hash table
    pub fn get_header(&self) -> HashHeader {
        self.header
    }

    /// Retrieve a single [`u32`] at `offset`
    fn get_u32(&self, offset: usize) -> Result<u32> {
        let bytes = self
            .data()?
            .get(offset..offset + size_of::<u32>())
            .ok_or(Error::DataOffset)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn bloom_words_offset(&self) -> usize {
        size_of::<HashHeader>()
    }

    fn bloom_words_end(&self) -> usize {
        self.bloom_words_offset() + self.header.bloom_words_len()
    }

    /// Returns the bloom words for this hash table
    #[allow(dead_code)]
    fn bloom_words(&self) -> Result<Option<&[u32]>> {
        // This indexing operation is safe as data is guaranteed to be larger than
        // bloom_words_offset and this will just return an empty slice if end == offset
        Ok(transmute_many_pedantic(
            &self.data()?[self.bloom_words_offset()..self.bloom_words_end()],
        )
        .ok())
    }

    fn get_bloom_word(&self, index: usize) -> Result<u32> {
        if index >= self.header.n_bloom_words() as usize {
            return Err(Error::DataOffset);
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

    /// The offset of the hash buckets section
    fn hash_buckets_offset(&self) -> usize {
        self.bloom_words_end()
    }

    /// The location where the hash bucket section ends
    fn hash_buckets_end(&self) -> usize {
        self.hash_buckets_offset() + self.header.buckets_len()
    }

    /// Return the hash value at `index`
    fn get_hash(&self, index: usize) -> Result<u32> {
        let start = self.hash_buckets_offset() + index * size_of::<u32>();
        self.get_u32(start)
    }

    /// The offset of the hash item section
    pub(crate) fn hash_items_offset(&self) -> usize {
        self.hash_buckets_end()
    }

    /// The number of hash items
    fn n_hash_items(&self) -> usize {
        let len = self.hash_items_end() - self.hash_items_offset();
        len / size_of::<HashItem>()
    }

    /// The location where the hash items section ends
    fn hash_items_end(&self) -> usize {
        self.pointer.size()
    }

    /// Get the hash item at hash item index
    fn get_hash_item_for_index(&self, index: usize) -> Result<HashItem> {
        let size = size_of::<HashItem>();
        let start = self.hash_items_offset() + size * index;
        let end = start + size;

        let data = self.data()?.get(start..end).ok_or(Error::DataOffset)?;
        Ok(transmute_one_pedantic(data)?)
    }

    /// Gets a list of keys contained in the hash table
    pub fn get_names(&self) -> Result<Vec<String>> {
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
                        return Err(Error::Data(format!(
                            "Parent with invalid offset encountered: {}",
                            parent
                        )));
                    }
                }
            }

            if last_inserted == inserted {
                // No insertion took place this round, there must be a parent loop
                // We fail instead of infinitely looping
                return Err(Error::Data(
                    "Error finding all parent items. The file appears to have a loop".to_string(),
                ));
            }
        }

        let names = names.into_iter().map(|s| s.unwrap()).collect();
        Ok(names)
    }

    /// Recurses through parents and check whether `item` has the specified full path name
    fn check_name(&self, item: &HashItem, key: &str) -> bool {
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

    /// Return the string that corresponds to the key part of the [`HashItem`]
    fn get_key(&self, item: &HashItem) -> Result<String> {
        let data = self.file.dereference(&item.key_ptr(), 1)?;
        Ok(String::from_utf8(data.to_vec())?)
    }

    /// Gets the item at key `key`
    pub fn get_hash_item(&self, key: &str) -> Result<HashItem> {
        if self.header.n_buckets() == 0 || self.n_hash_items() == 0 {
            return Err(Error::KeyNotFound(key.to_string()));
        }

        let hash_value = djb_hash(key);
        if !self.bloom_filter(hash_value) {
            return Err(Error::KeyNotFound(key.to_string()));
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

        Err(Error::KeyNotFound(key.to_string()))
    }

    /// Get the bytes for the [`HashItem`] at `key`
    fn get_bytes(&self, key: &str) -> Result<&[u8]> {
        let item = self.get_hash_item(key)?;
        let typ = item.typ()?;
        if typ == HashItemType::Value {
            Ok(self.file.dereference(item.value_ptr(), 8)?)
        } else {
            Err(Error::Data(format!(
                "Unable to parse item for key '{}' as GVariant: Expected type 'v', got type {}",
                self.get_key(&item)?,
                typ
            )))
        }
    }

    /// Get the item at key `key` and try to interpret it as a [`HashTable`]
    pub fn get_hash_table(&self, key: &str) -> Result<HashTable> {
        let item = self.get_hash_item(key)?;
        let typ = item.typ()?;
        if typ == HashItemType::HashTable {
            HashTable::for_bytes(*item.value_ptr(), self.file)
        } else {
            Err(Error::Data(format!(
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type '{}'",
                self.get_key(&item)?,
                typ
            )))
        }
    }

    fn deserializer_for_key(&self, key: &str) -> Result<GVariantDeserializer> {
        let data = self.get_bytes(key)?;

        // Create a new zvariant context based our endianess and the byteswapped property
        let context =
            zvariant::serialized::Context::new_gvariant(self.file.zvariant_endianess(), 0);

        // On non-unix systems this function lacks the FD argument
        let de: GVariantDeserializer = GVariantDeserializer::new(
            data,
            #[cfg(unix)]
            None::<&[zvariant::Fd]>,
            zvariant::Value::signature(),
            context,
        )?;

        Ok(de)
    }

    /// Get the data at key `key` as a [`enum@zvariant::Value`]
    ///
    /// Unless you need to inspect the value at runtime, it is recommended to use [`HashTable::get`]
    pub fn get_value(&self, key: &str) -> Result<zvariant::Value> {
        let mut de = self.deserializer_for_key(key)?;
        Ok(zvariant::Value::deserialize(&mut de)?)
    }

    /// Get the data at key `key` and try to deserialize a [`enum@zvariant::Value`]
    ///
    /// Then try to extract an underlying `T`
    pub fn get<'d, T>(&'d self, key: &str) -> Result<T>
    where
        T: zvariant::Type + serde::Deserialize<'d> + 'd,
    {
        let mut de = self.deserializer_for_key(key)?;
        let value = zvariant::DeserializeValue::deserialize(&mut de).map_err(|err| {
            Error::Data(format!(
                "Error deserializing value for key \"{}\" as gvariant type \"{}\": {}",
                key,
                T::signature(),
                err
            ))
        })?;

        Ok(value.0)
    }

    /// Get the data at key `key` as a [`struct@glib::Variant`]
    #[cfg(feature = "glib")]
    /// Get the item at key `key` and try to interpret it as a [`struct@glib::Variant`]
    pub fn get_gvariant(&self, key: &str) -> Result<glib::Variant> {
        let data = self.get_bytes(key)?;
        let variant = glib::Variant::from_data_with_type(data, glib::VariantTy::VARIANT);

        if self.file.byteswapped {
            Ok(variant.byteswap())
        } else {
            Ok(variant)
        }
    }
}

impl std::fmt::Debug for HashTable<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashTable")
            .field("header", &self.header)
            .field(
                "map",
                &self.get_names().map(|res| {
                    res.iter()
                        .map(|name| {
                            let item = self.get_hash_item(name);
                            match item {
                                Ok(item) => {
                                    let value = match item.typ() {
                                        Ok(super::HashItemType::Container) => {
                                            Ok(Box::new(item) as Box<dyn std::fmt::Debug>)
                                        }
                                        Ok(super::HashItemType::HashTable) => {
                                            self.get_hash_table(name).map(|table| {
                                                Box::new(table) as Box<dyn std::fmt::Debug>
                                            })
                                        }
                                        Ok(super::HashItemType::Value) => {
                                            self.get_value(name).map(|value| {
                                                Box::new(value) as Box<dyn std::fmt::Debug>
                                            })
                                        }
                                        Err(err) => Err(err),
                                    };

                                    (name.to_string(), Ok((item, value)))
                                }
                                Err(err) => (name.to_string(), Err(err)),
                            }
                        })
                        .collect::<std::collections::HashMap<_, _>>()
                }),
            )
            .finish()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::read::{Error, File, HashHeader, HashItem, Pointer};
    use crate::test::*;
    use crate::test::{assert_eq, assert_matches, assert_ne};

    #[test]
    fn debug() {
        let header = HashHeader::new(0, 0, 0);
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
        println!("{:?}", table);
    }

    #[test]
    fn bloom_words() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let header = table.get_header();
        assert_eq!(header.n_bloom_words(), 0);
        assert_eq!(header.bloom_words_len(), 0);
        assert_eq!(table.bloom_words().unwrap(), None);
    }

    #[test]
    fn get_item() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let res = table.get_hash_item("test");
        assert_matches!(res, Err(Error::KeyNotFound(_)));

        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let item = table.get_hash_item("test").unwrap();
            assert_ne!(item.value_ptr(), &Pointer::NULL);
            let value: String = table.get_value("test").unwrap().try_into().unwrap();
            assert_eq!(value, "test");

            let item_fail = table.get_hash_item("fail").unwrap_err();
            assert_matches!(item_fail, Error::KeyNotFound(_));

            let res_item = table.get_hash_item("test_fail");
            assert_matches!(res_item, Err(Error::KeyNotFound(_)));
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
            assert_matches!(res, Err(Error::Data(_)));
        }
    }

    #[test]
    fn get_bloom_word() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res = table.get_bloom_word(0);
            assert_matches!(res, Err(Error::DataOffset));
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
            assert_matches!(fail, Error::KeyNotFound(_));
        }
    }

    #[test]
    fn get_hash_table() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let table = table.get_hash_table("table").unwrap();
        let fail = table.get_hash_table("fail").unwrap_err();
        assert_matches!(fail, Error::KeyNotFound(_));
    }

    #[test]
    fn check_name_pass() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let item = table.get_hash_item("string").unwrap();
        assert_eq!(table.check_name(&item, "string"), true);
    }

    #[test]
    fn check_name_invalid_name() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let item = table.get_hash_item("string").unwrap();
        assert_eq!(table.check_name(&item, "fail"), false);
    }

    #[test]
    fn check_name_wrong_item() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let table = table.get_hash_table("table").unwrap();

        // Get an item from the sub-hash table and call check_names on the root
        let item = table.get_hash_item("int").unwrap();
        assert_eq!(table.check_name(&item, "table"), false);
    }

    #[test]
    fn check_name_broken_key_pointer() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let table = table.get_hash_table("table").unwrap();

        // Break the key pointer
        let item = table.get_hash_item("int").unwrap();
        let key_ptr = Pointer::new(500, 500);
        let broken_item = HashItem::new(
            item.hash_value(),
            item.parent(),
            key_ptr,
            item.typ().unwrap(),
            item.value_ptr().clone(),
        );

        assert_eq!(table.check_name(&broken_item, "table"), false);
    }

    #[test]
    fn check_name_invalid_parent() {
        let file = File::from_file(&TEST_FILE_3).unwrap();
        let table = file.hash_table().unwrap();

        // Break the key pointer
        let item = table
            .get_hash_item("/gvdb/rs/test/online-symbolic.svg")
            .unwrap();
        let broken_item = HashItem::new(
            item.hash_value(),
            50,
            item.key_ptr(),
            item.typ().unwrap(),
            item.value_ptr().clone(),
        );

        assert_eq!(
            table.check_name(&broken_item, "/gvdb/rs/test/online-symbolic.svg"),
            false
        );
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::Error;
    use crate::test::new_simple_file;
    use glib::prelude::*;
    use matches::assert_matches;

    #[test]
    fn get_gvariant() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res: glib::Variant = table.get_gvariant("test").unwrap().get().unwrap();
            assert_eq!(&res, &"test".to_variant());

            let fail = table.get_gvariant("fail").unwrap_err();
            assert_matches!(fail, Error::KeyError(_));
        }
    }
}
