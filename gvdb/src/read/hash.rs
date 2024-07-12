use crate::read::error::{Error, Result};
use crate::read::hash_item::HashItem;
use crate::util::djb_hash;
use safe_transmute::transmute_many_pedantic;
use serde::Deserialize;
use std::cmp::min;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};
use zvariant::Type;

use super::slice::SliceLEu32;
use super::{File, HashItemType};

#[cfg(unix)]
type GVariantDeserializer<'de, 'sig, 'f> =
    zvariant::gvariant::Deserializer<'de, 'sig, 'f, zvariant::Fd<'f>>;
#[cfg(not(unix))]
type GVariantDeserializer<'de, 'sig, 'f> = zvariant::gvariant::Deserializer<'de, 'sig, 'f, ()>;

/// The header of a GVDB hash table.
///
/// ```text
/// +-------+-----------------------+
/// | Bytes | Field                 |
/// +-------+-----------------------+
/// |     4 | number of bloom words |
/// +-------+-----------------------+
/// |     4 | number of buckets     |
/// +-------+-----------------------+
/// ```
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, FromBytes, FromZeroes, AsBytes)]
pub struct HashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

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

    /// Read the hash table header from `data`
    pub fn try_from_bytes(data: &[u8]) -> Result<Self> {
        HashHeader::read_from_prefix(data)
            .ok_or(Error::Data("Invalid hash table header".to_string()))
    }

    /// Number of bloom words in the hash table header
    pub fn n_bloom_words(&self) -> u32 {
        u32::from_le(self.n_bloom_words) & ((1 << 27) - 1)
    }

    /// The start of the bloom words region
    pub fn bloom_words_offset(&self) -> usize {
        size_of::<Self>()
    }

    /// Size of the bloom words section in the header
    pub fn bloom_words_len(&self) -> usize {
        self.n_bloom_words() as usize * size_of::<u32>()
    }

    /// Read the bloom words from `data`
    fn read_bloom_words<'a>(&self, data: &'a [u8]) -> Result<SliceLEu32<'a>> {
        // Bloom words come directly after header
        let offset = self.bloom_words_offset();
        let len = self.bloom_words_len();

        if len == 0 {
            Ok(SliceLEu32(&[]))
        } else {
            let words_data = data.get(offset..(offset + len)).ok_or_else(|| {
                Error::Data(format!(
                    "Not enough bytes to fit hash table: Expected at least {} bytes, got {}",
                    self.items_offset(),
                    data.len()
                ))
            })?;
            Ok(SliceLEu32(
                transmute_many_pedantic(words_data).or(Err(Error::DataOffset))?,
            ))
        }
    }

    /// The offset of the hash buckets section
    pub fn buckets_offset(&self) -> usize {
        self.bloom_words_offset() + self.bloom_words_len()
    }

    /// Number of hash buckets in the hash table header
    pub fn n_buckets(&self) -> u32 {
        u32::from_le(self.n_buckets)
    }

    /// Length of the hash buckets section in the header
    pub fn buckets_len(&self) -> usize {
        self.n_buckets() as usize * size_of::<u32>()
    }

    /// Read the buckets as a little endian slice
    fn read_buckets<'a>(&self, data: &'a [u8]) -> Result<SliceLEu32<'a>> {
        let offset = self.buckets_offset();
        let len = self.buckets_len();

        if len == 0 {
            Ok(SliceLEu32(&[]))
        } else {
            let buckets_data = data.get(offset..(offset + len)).ok_or_else(|| {
                Error::Data(format!(
                    "Not enough bytes to fit hash table: Expected at least {} bytes, got {}",
                    self.items_offset(),
                    data.len()
                ))
            })?;
            Ok(SliceLEu32(
                transmute_many_pedantic(buckets_data).or(Err(Error::DataOffset))?,
            ))
        }
    }

    /// The start of the hash items region
    pub fn items_offset(&self) -> usize {
        self.buckets_offset() + self.buckets_len()
    }

    /// Read the items as a slice
    fn read_items<'a>(&self, data: &'a [u8]) -> Result<&'a [HashItem]> {
        let offset = self.items_offset();
        let len = data.len().saturating_sub(offset);

        if len == 0 {
            // The hash table has no items. This is generally valid.
            Ok(&[])
        } else if len % size_of::<HashItem>() != 0 {
            Err(Error::Data(format!(
                "Hash item size invalid: Expected a multiple of {}, got {}",
                size_of::<HashItem>(),
                data.len()
            )))
        } else {
            let items_data = data.get(offset..(offset + len)).unwrap_or_default();
            Ok(transmute_many_pedantic(items_data).or(Err(Error::DataOffset))?)
        }
    }
}

impl Debug for HashHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashHeader")
            .field("n_bloom_words", &self.n_bloom_words())
            .field("n_buckets", &self.n_buckets())
            .field("data", &self.as_bytes())
            .finish()
    }
}

/// A hash table inside a GVDB file
///
/// ```text
/// +--------+---------------------------+
/// |  Bytes | Field                     |
/// +--------+---------------------------+
/// |      4 | number of bloom words (b) |
/// +--------+---------------------------+
/// |      4 | number of buckets (n)     |
/// +--------+---------------------------+
/// |  b * 4 | bloom words               |
/// +--------+---------------------------+
/// |  n * 4 | buckets                   |
/// +--------+---------------------------+
/// | x * 24 | hash items                |
/// +--------+---------------------------+
/// ```
#[derive(Clone)]
pub struct HashTable<'a, 'file> {
    pub(crate) file: &'a File<'file>,
    pub(crate) header: HashHeader,
    bloom_words: SliceLEu32<'a>,
    buckets: SliceLEu32<'a>,
    items: &'a [HashItem],
}

impl<'a, 'file> HashTable<'a, 'file> {
    /// Interpret a chunk of bytes as a HashTable. The table_ptr should point to the hash table.
    /// Data has to be the complete GVDB file, as hash table items are stored somewhere else.
    pub(crate) fn for_bytes(data: &'a [u8], root: &'a File<'file>) -> Result<Self> {
        let header = HashHeader::try_from_bytes(data)?;
        let bloom_words = header.read_bloom_words(data)?;
        let buckets = header.read_buckets(data)?;
        let items = header.read_items(data)?;

        Ok(Self {
            file: root,
            header,
            bloom_words,
            buckets,
            items,
        })
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
        let bloom_word = self.bloom_words.get(word as usize).unwrap();
        bloom_word & mask == mask
    }

    /// Get the hash item at hash item index
    fn get_hash_item_for_index(&self, index: usize) -> Option<&HashItem> {
        self.items.get(index)
    }

    /// Gets a list of keys contained in the hash table.
    pub fn keys(&self) -> Result<Vec<String>> {
        let count = self.items.len();
        let mut names: Vec<Option<String>> = vec![None; count];

        let mut inserted = 0;
        while inserted < count {
            let last_inserted = inserted;
            for index in 0..count {
                let item = self
                    .get_hash_item_for_index(index)
                    .ok_or(Error::DataOffset)?;
                let parent: usize = item.parent().try_into()?;

                if names[index].is_none() {
                    // Only process items not already processed
                    if parent == 0xffffffff {
                        // root item
                        let name = self.key_for_item(item)?;
                        names[index] = Some(name.to_string());
                        inserted += 1;
                    } else if parent < count && names[parent].is_some() {
                        // We already came across this item
                        let name = self.key_for_item(item)?;
                        let parent_name = names.get(parent).unwrap().as_ref().unwrap();
                        let full_name = parent_name.to_string() + name;
                        names[index] = Some(full_name);
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
    fn check_key(&self, item: &HashItem, key: &str) -> bool {
        let this_key = match self.key_for_item(item) {
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

        if parent < self.items.len() as u32 && !key.is_empty() {
            let parent_item = match self.get_hash_item_for_index(parent as usize) {
                Some(p) => p,
                None => return false,
            };

            let parent_key_len = key.len() - this_key.len();
            return self.check_key(parent_item, &key[0..parent_key_len]);
        }

        false
    }

    /// Return the string that corresponds to the key part of the [`HashItem`].
    fn key_for_item(&self, item: &HashItem) -> Result<&str> {
        let data = self.file.dereference(&item.key_ptr(), 1)?;
        Ok(std::str::from_utf8(data)?)
    }

    /// Gets the item at key `key`.
    pub(crate) fn get_hash_item(&self, key: &str) -> Option<HashItem> {
        if self.header.n_buckets() == 0 || self.items.is_empty() {
            return None;
        }

        let hash_value = djb_hash(key);
        if !self.bloom_filter(hash_value) {
            return None;
        }

        let bucket = hash_value % self.buckets.len() as u32;
        let mut itemno = self.buckets.get(bucket as usize)? as usize;

        let lastno = if bucket == self.header.n_buckets() - 1 {
            self.items.len()
        } else {
            min(
                self.buckets.get(bucket as usize + 1)?,
                self.items.len() as u32,
            ) as usize
        };

        while itemno < lastno {
            let item = self.get_hash_item_for_index(itemno)?;
            if hash_value == item.hash_value() && self.check_key(item, key) {
                return Some(*item);
            }

            itemno += 1;
        }

        None
    }

    /// Get the bytes for the [`HashItem`] at `key`.
    fn get_bytes(&self, key: &str) -> Result<&[u8]> {
        let item = self
            .get_hash_item(key)
            .ok_or(Error::KeyNotFound(key.to_string()))?;
        let typ = item.typ()?;
        if typ == HashItemType::Value {
            Ok(self.file.dereference(item.value_ptr(), 8)?)
        } else {
            Err(Error::Data(format!(
                "Unable to parse item for key '{}' as GVariant: Expected type 'v', got type {}",
                self.key_for_item(&item)?,
                typ
            )))
        }
    }

    /// Returns the nested [`HashTable`] at `key`, if one is found.
    pub fn get_hash_table(&self, key: &str) -> Result<HashTable> {
        let item = self
            .get_hash_item(key)
            .ok_or(Error::KeyNotFound(key.to_string()))?;
        let typ = item.typ()?;
        if typ == HashItemType::HashTable {
            self.file.read_hash_table(item.value_ptr())
        } else {
            Err(Error::Data(format!(
                "Unable to parse item for key '{}' as hash table: Expected type 'H', got type '{}'",
                self.key_for_item(&item)?,
                typ
            )))
        }
    }

    /// Create a zvariant deserializer for the specified key.
    fn deserializer_for_key(&self, key: &str) -> Result<GVariantDeserializer> {
        let data = self.get_bytes(key)?;

        // Create a new zvariant context based on our endianess and the byteswapped property
        let context = zvariant::serialized::Context::new_gvariant(self.file.endianness(), 0);

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

    /// Returns the data for `key` as a [`enum@zvariant::Value`].
    ///
    /// Unless you need to inspect the value at runtime, it is recommended to use [`HashTable::get`].
    pub fn get_value(&self, key: &str) -> Result<zvariant::Value> {
        let mut de = self.deserializer_for_key(key)?;
        Ok(zvariant::Value::deserialize(&mut de)?)
    }

    /// Returns the data for `key` and try to deserialize a [`enum@zvariant::Value`].
    ///
    /// Then try to extract an underlying `T`.
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

    #[cfg(feature = "glib")]
    /// Returns the data for `key` as a [`struct@glib::Variant`].
    pub fn get_gvariant(&self, key: &str) -> Result<glib::Variant> {
        let data = self.get_bytes(key)?;
        let variant = glib::Variant::from_data_with_type(data, glib::VariantTy::VARIANT);

        if self.file.endianness == zvariant::Endian::native() {
            Ok(variant)
        } else {
            Ok(variant.byteswap())
        }
    }
}

impl std::fmt::Debug for HashTable<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashTable")
            .field("header", &self.header)
            .field("bloom_words", &self.bloom_words)
            .field("buckets", &self.buckets)
            .field(
                "map",
                &self.keys().map(|res| {
                    res.iter()
                        .map(|name| {
                            let item = self.get_hash_item(name);
                            match item {
                                Some(item) => {
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

                                    (name.to_string(), Some((item, value)))
                                }
                                None => (name.to_string(), None),
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
        let header2 = header;
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
        let header = table.header;
        assert_eq!(header.n_buckets(), 0);

        let file = new_simple_file(false);
        let table = file.hash_table().unwrap();
        let header = table.header;
        assert_eq!(header.n_buckets(), 1);
        println!("{:?}", table);
    }

    #[test]
    fn bloom_words() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let header = table.header;
        assert_eq!(header.n_bloom_words(), 0);
        assert_eq!(header.bloom_words_len(), 0);
        assert!(table.bloom_words.is_empty());
    }

    #[test]
    fn get_item() {
        let file = new_empty_file();
        let table = file.hash_table().unwrap();
        let res = table.get_hash_item("test");
        assert_matches!(res, None);

        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let item = table.get_hash_item(SIMPLE_FILE_KEY).unwrap();
            assert_ne!(item.value_ptr(), &Pointer::NULL);
            let value: u32 = table
                .get_value(SIMPLE_FILE_KEY)
                .unwrap()
                .try_into()
                .unwrap();
            assert_eq!(value, SIMPLE_FILE_VALUE);

            let item_fail = table.get_hash_item("fail");
            assert_matches!(item_fail, None);

            let res_item = table.get_hash_item("test_fail");
            assert_matches!(res_item, None);
        }
    }

    #[test]
    fn get() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res: u32 = table.get::<u32>(SIMPLE_FILE_KEY).unwrap();
            assert_eq!(res, SIMPLE_FILE_VALUE);

            let res = table.get::<i32>(SIMPLE_FILE_KEY);
            assert_matches!(res, Err(Error::Data(_)));
        }
    }

    #[test]
    fn get_bloom_word() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res = table.bloom_words.get(0);
            assert_matches!(res, None);
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
            let res = table.get_value(SIMPLE_FILE_KEY).unwrap();
            assert_eq!(&res, &zvariant::Value::from(SIMPLE_FILE_VALUE));

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
        assert_eq!(table.check_key(&item, "string"), true);
    }

    #[test]
    fn check_name_invalid_name() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let item = table.get_hash_item("string").unwrap();
        assert_eq!(table.check_key(&item, "fail"), false);
    }

    #[test]
    fn check_name_wrong_item() {
        let file = File::from_file(&TEST_FILE_2).unwrap();
        let table = file.hash_table().unwrap();
        let table = table.get_hash_table("table").unwrap();

        // Get an item from the sub-hash table and call check_names on the root
        let item = table.get_hash_item("int").unwrap();
        assert_eq!(table.check_key(&item, "table"), false);
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
            *item.value_ptr(),
        );

        assert_eq!(table.check_key(&broken_item, "table"), false);
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
            *item.value_ptr(),
        );

        assert_eq!(
            table.check_key(&broken_item, "/gvdb/rs/test/online-symbolic.svg"),
            false
        );
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::Error;
    use crate::test::{new_simple_file, SIMPLE_FILE_KEY, SIMPLE_FILE_VALUE};
    use glib::prelude::*;
    use matches::assert_matches;

    #[test]
    fn get_gvariant() {
        for endianess in [true, false] {
            let file = new_simple_file(endianess);
            let table = file.hash_table().unwrap();
            let res: glib::Variant = table.get_gvariant(SIMPLE_FILE_KEY).unwrap().get().unwrap();
            assert_eq!(res, SIMPLE_FILE_VALUE.to_variant());

            let fail = table.get_gvariant("fail").unwrap_err();
            assert_matches!(fail, Error::KeyNotFound(_));
        }
    }
}
