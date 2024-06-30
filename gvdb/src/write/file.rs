use crate::read::HashHeader;
use crate::read::HashItem;
use crate::read::Header;
use crate::read::Pointer;
use crate::util::align_offset;
use crate::write::error::{Error, Result};
use crate::write::hash::SimpleHashTable;
use crate::write::item::HashValue;
use safe_transmute::transmute_one_to_bytes;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::mem::size_of;

/// Create hash tables for use in GVDB files
///
/// # Example
///
/// ```
/// use glib::prelude::*;
/// use gvdb::write::{FileWriter, HashTableBuilder};
///
/// let file_writer = FileWriter::new();
/// let mut table_builder = HashTableBuilder::new();
/// table_builder
///     .insert_string("string", "test string")
///     .unwrap();
/// let gvdb_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
/// ```
#[derive(Debug)]
pub struct HashTableBuilder<'a> {
    items: HashMap<String, HashValue<'a>>,
    path_separator: Option<String>,
}

impl<'a> HashTableBuilder<'a> {
    /// Create a new empty HashTableBuilder with the default path separator `/`
    ///
    /// ```
    /// # use gvdb::write::HashTableBuilder;
    /// let mut table_builder = HashTableBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::with_path_separator(Some("/"))
    }

    /// Create a new empty HashTableBuilder a different path separator than `/` or none at all
    ///
    /// ```
    /// # use gvdb::write::HashTableBuilder;
    /// let mut table_builder = HashTableBuilder::with_path_separator(Some(":"));
    /// ```
    pub fn with_path_separator(sep: Option<&str>) -> Self {
        Self {
            items: Default::default(),
            path_separator: sep.map(|s| s.to_string()),
        }
    }

    /// Insert the provided [`HashValue`] for the key.
    fn insert_item_value(
        &mut self,
        key: &(impl ToString + ?Sized),
        item: HashValue<'a>,
    ) -> Result<()> {
        let key = key.to_string();

        if let Some(sep) = &self.path_separator {
            let mut this_key = "".to_string();
            let mut last_key: Option<String> = None;

            for segment in key.split(sep) {
                this_key += segment;
                if this_key != key {
                    this_key += sep;
                }

                if let Some(last_key) = last_key {
                    if let Some(last_item) = self.items.get_mut(&last_key) {
                        if let HashValue::Container(ref mut container) = last_item {
                            if !container.contains(&this_key) {
                                container.push(this_key.clone());
                            }
                        } else {
                            return Err(Error::Consistency(format!(
                                "Parent item with key '{}' is not of type container",
                                this_key
                            )));
                        }
                    } else {
                        let parent_item = HashValue::Container(vec![this_key.clone()]);
                        self.items.insert(last_key.to_string(), parent_item);
                    }
                }

                if key == this_key {
                    // The item we actually want to insert
                    self.items.insert(key.to_string(), item);
                    break;
                }

                last_key = Some(this_key.clone());
            }
        } else {
            self.items.insert(key, item);
        }

        Ok(())
    }

    /// Insert Value `item` for `key`
    ///
    /// ```
    /// use zvariant::Value;
    /// let mut table_builder = gvdb::write::HashTableBuilder::new();
    /// let variant = Value::new(123u32);
    /// table_builder.insert_value("variant_123", variant);
    /// ```
    pub fn insert_value(
        &mut self,
        key: &(impl ToString + ?Sized),
        value: zvariant::Value<'a>,
    ) -> Result<()> {
        let item = HashValue::Value(value);
        self.insert_item_value(key, item)
    }

    /// Insert `item` for `key` where item needs to be `Into<zvariant::Value>`
    ///
    /// ```
    /// use zvariant::Value;
    /// let mut table_builder = gvdb::write::HashTableBuilder::new();
    /// let value = 123u32;
    /// table_builder.insert("variant_123", value);
    /// ```
    pub fn insert<T>(&mut self, key: &(impl ToString + ?Sized), value: T) -> Result<()>
    where
        T: Into<zvariant::Value<'a>>,
    {
        let item = HashValue::Value(value.into());
        self.insert_item_value(key, item)
    }

    /// Insert GVariant `item` for `key`
    ///
    /// ```
    /// # #[cfg(feature = "glib")]
    /// # use glib::prelude::*;
    /// #
    /// let mut table_builder = gvdb::write::HashTableBuilder::new();
    /// let variant = 123u32.to_variant();
    /// table_builder.insert_gvariant("variant_123", variant);
    /// ```
    #[cfg(feature = "glib")]
    pub fn insert_gvariant(
        &mut self,
        key: &(impl ToString + ?Sized),
        variant: glib::Variant,
    ) -> Result<()> {
        let item = HashValue::GVariant(variant);
        self.insert_item_value(key, item)
    }

    /// Convenience method to create a string type GVariant for `value` and insert it at `key`
    ///
    /// ```
    /// # let mut table_builder = gvdb::write::HashTableBuilder::new();
    /// table_builder.insert_string("string_key", "string_data");
    /// ```
    pub fn insert_string(
        &mut self,
        key: &(impl ToString + ?Sized),
        string: &(impl ToString + ?Sized),
    ) -> Result<()> {
        let variant = zvariant::Value::new(string.to_string());
        self.insert_value(key, variant)
    }

    /// Convenience method to create a byte type GVariant for `value` and insert it at `key`
    ///
    /// ```
    /// # let mut table_builder = gvdb::write::HashTableBuilder::new();
    /// table_builder.insert_bytes("bytes", &[1, 2, 3, 4, 5]);
    /// ```
    pub fn insert_bytes(&mut self, key: &(impl ToString + ?Sized), bytes: &'a [u8]) -> Result<()> {
        let value = zvariant::Value::new(bytes);
        self.insert_value(key, value)
    }

    /// Insert an entire hash table at `key`.
    ///
    /// ```
    /// # use zvariant::Value;
    /// # use gvdb::write::HashTableBuilder;
    /// let mut table_builder = HashTableBuilder::new();
    /// let mut table_builder_2 = HashTableBuilder::new();
    /// table_builder_2
    ///     .insert_value("int", Value::new(42u32))
    ///     .unwrap();
    ///
    /// table_builder
    ///     .insert_table("table", table_builder_2)
    ///     .unwrap();
    /// ```
    pub fn insert_table(
        &mut self,
        key: &(impl ToString + ?Sized),
        table_builder: HashTableBuilder<'a>,
    ) -> Result<()> {
        let item = HashValue::TableBuilder(table_builder);
        self.insert_item_value(key, item)
    }

    /// The number of items contained in the hash table builder
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the hash table builder contains no items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn build(mut self) -> Result<SimpleHashTable<'a>> {
        let mut hash_table = SimpleHashTable::with_n_buckets(self.items.len());

        let mut keys: Vec<String> = self.items.keys().cloned().collect();
        keys.sort();

        for key in keys {
            let value = self.items.remove(&key).unwrap();
            hash_table.insert(&key, value);
        }

        for (key, item) in hash_table.iter() {
            if let HashValue::Container(container) = &*item.value_ref() {
                for child in container {
                    let child_item = hash_table.get(child);
                    if let Some(child_item) = child_item {
                        child_item.parent().replace(Some(item.clone()));
                    } else {
                        return Err(Error::Consistency(format!("Tried to set parent for child '{}' to '{}' but the child was not found.", child, key)));
                    }
                }
            }
        }

        Ok(hash_table)
    }
}

impl<'a> Default for HashTableBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct Chunk {
    // The pointer that points to the data where the chunk will be in memory in the finished file
    pointer: Pointer,

    // We use a boxed slice because this guarantees that the size is not changed afterwards
    data: Box<[u8]>,
}

impl Chunk {
    pub fn new(data: Box<[u8]>, pointer: Pointer) -> Self {
        Self { pointer, data }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Box<[u8]> {
        self.data
    }

    pub fn pointer(&self) -> Pointer {
        self.pointer
    }
}

/// Create GVDB files
///
/// # Example
/// ```
/// use glib::prelude::*;
/// use gvdb::write::{FileWriter, HashTableBuilder};
///
/// fn create_gvdb_file() {
///     let mut file_writer = FileWriter::new();
///     let mut table_builder = HashTableBuilder::new();
///     table_builder
///            .insert_string("string", "test string")
///            .unwrap();
///     let file_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
/// }
/// ```
pub struct FileWriter {
    offset: usize,
    chunks: VecDeque<Chunk>,
    byteswap: bool,
}

impl FileWriter {
    /// Create a new instance configured for writing little endian data (preferred endianness)
    /// ```
    /// let file_writer = gvdb::write::FileWriter::new();
    /// ```
    pub fn new() -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = false;
        #[cfg(target_endian = "big")]
        let byteswap = true;
        Self::with_byteswap(byteswap)
    }

    /// Create a new instance configured for writing big endian data
    /// (not recommended for most use cases)
    /// ```
    /// let file_writer = gvdb::write::FileWriter::new();
    /// ```
    pub fn for_big_endian() -> Self {
        #[cfg(target_endian = "little")]
        let byteswap = true;
        #[cfg(target_endian = "big")]
        let byteswap = false;
        Self::with_byteswap(byteswap)
    }

    /// Specify manually whether you want to swap the endianness of the file. The default is to
    /// always create a little-endian file
    fn with_byteswap(byteswap: bool) -> Self {
        let mut this = Self {
            offset: 0,
            chunks: Default::default(),
            byteswap,
        };

        this.allocate_empty_chunk(size_of::<Header>(), 1);
        this
    }

    /// Allocate a chunk
    fn allocate_chunk_with_data(
        &mut self,
        data: Box<[u8]>,
        alignment: usize,
    ) -> (usize, &mut Chunk) {
        // Align the data
        self.offset = align_offset(self.offset, alignment);

        // Calculate the pointer
        let offset_start = self.offset;
        let offset_end = offset_start + data.len();
        let pointer = Pointer::new(offset_start, offset_end);

        // Update the offset to the end of the chunk
        self.offset = offset_end;

        let chunk = Chunk::new(data, pointer);
        self.chunks.push_back(chunk);
        let index = self.chunks.len() - 1;
        (index, &mut self.chunks[index])
    }

    fn allocate_empty_chunk(&mut self, size: usize, alignment: usize) -> (usize, &mut Chunk) {
        let data = vec![0; size].into_boxed_slice();
        self.allocate_chunk_with_data(data, alignment)
    }

    fn add_value(&mut self, value: &zvariant::Value) -> Result<(usize, &mut Chunk)> {
        #[cfg(target_endian = "little")]
        let le = true;
        #[cfg(target_endian = "big")]
        let le = false;

        let data: Box<[u8]> = if le && !self.byteswap || !le && self.byteswap {
            let context = zvariant::serialized::Context::new_gvariant(zvariant::LE, 0);
            Box::from(&*zvariant::to_bytes(context, value)?)
        } else {
            let context = zvariant::serialized::Context::new_gvariant(zvariant::BE, 0);
            Box::from(&*zvariant::to_bytes(context, value)?)
        };

        Ok(self.allocate_chunk_with_data(data, 8))
    }

    #[cfg(feature = "glib")]
    fn add_gvariant(&mut self, variant: &glib::Variant) -> (usize, &mut Chunk) {
        let value = if self.byteswap {
            glib::Variant::from_variant(&variant.byteswap())
        } else {
            glib::Variant::from_variant(variant)
        };

        let normal = value.normal_form();
        let data = normal.data();
        self.allocate_chunk_with_data(data.to_vec().into_boxed_slice(), 8)
    }

    fn add_string(&mut self, string: &str) -> (usize, &mut Chunk) {
        let data = string.to_string().into_boxed_str().into_boxed_bytes();
        self.allocate_chunk_with_data(data, 1)
    }

    fn add_simple_hash_table(&mut self, table: SimpleHashTable) -> Result<(usize, &mut Chunk)> {
        for (index, (_bucket, item)) in table.iter().enumerate() {
            item.set_assigned_index(index as u32);
        }

        let header = HashHeader::new(5, 0, table.n_buckets() as u32);
        let items_len = table.n_items() * size_of::<HashItem>();
        let size =
            size_of::<HashHeader>() + header.bloom_words_len() + header.buckets_len() + items_len;

        let hash_buckets_offset = size_of::<HashHeader>() + header.bloom_words_len();
        let hash_items_offset = hash_buckets_offset + header.buckets_len();

        let (hash_table_chunk_index, hash_table_chunk) = self.allocate_empty_chunk(size, 4);
        let header = transmute_one_to_bytes(&header);
        hash_table_chunk.data_mut()[0..header.len()].copy_from_slice(header);

        let mut n_item = 0;
        for bucket in 0..table.n_buckets() {
            let hash_bucket_start = hash_buckets_offset + bucket * size_of::<u32>();
            let hash_bucket_end = hash_bucket_start + size_of::<u32>();

            self.chunks[hash_table_chunk_index].data[hash_bucket_start..hash_bucket_end]
                .copy_from_slice(u32::to_le_bytes(n_item as u32).as_slice());

            for current_item in table.iter_bucket(bucket) {
                let parent = if let Some(parent) = &*current_item.parent_ref() {
                    parent.assigned_index()
                } else {
                    u32::MAX
                };

                let key = if let Some(parent) = &*current_item.parent_ref() {
                    current_item.key().strip_prefix(parent.key()).unwrap_or("")
                } else {
                    current_item.key()
                };

                if key.is_empty() {
                    return Err(Error::Consistency(format!(
                        "Item '{}' already exists in hash map or key is empty",
                        current_item.key()
                    )));
                }

                let key_ptr = self.add_string(key).1.pointer();
                let typ = current_item.value_ref().typ();

                let value_ptr = match current_item.value().take() {
                    HashValue::Value(value) => self.add_value(&value)?.1.pointer(),
                    #[cfg(feature = "glib")]
                    HashValue::GVariant(variant) => self.add_gvariant(&variant).1.pointer(),
                    HashValue::TableBuilder(tb) => self.add_table_builder(tb)?.1.pointer(),
                    HashValue::Container(children) => {
                        let size = children.len() * size_of::<u32>();
                        let chunk = self.allocate_empty_chunk(size, 4).1;

                        let mut offset = 0;
                        for child in children {
                            let child_item = table.get(&child);
                            if let Some(child_item) = child_item {
                                child_item.parent().replace(Some(current_item.clone()));

                                chunk.data_mut()[offset..offset + size_of::<u32>()]
                                    .copy_from_slice(&u32::to_le_bytes(
                                        child_item.assigned_index(),
                                    ));
                                offset += size_of::<u32>();
                            } else {
                                return Err(Error::Consistency(format!(
                                    "Child item '{}' not found for parent: '{}'",
                                    child, key
                                )));
                            }
                        }

                        chunk.pointer()
                    }
                };

                let hash_item = HashItem::new(current_item.hash(), parent, key_ptr, typ, value_ptr);

                let hash_item_start = hash_items_offset + n_item * size_of::<HashItem>();
                let hash_item_end = hash_item_start + size_of::<HashItem>();

                self.chunks[hash_table_chunk_index].data[hash_item_start..hash_item_end]
                    .copy_from_slice(transmute_one_to_bytes(&hash_item));

                n_item += 1;
            }
        }

        Ok((
            hash_table_chunk_index,
            &mut self.chunks[hash_table_chunk_index],
        ))
    }

    fn add_table_builder(
        &mut self,
        table_builder: HashTableBuilder,
    ) -> Result<(usize, &mut Chunk)> {
        self.add_simple_hash_table(table_builder.build()?)
    }

    fn file_size(&self) -> usize {
        self.chunks[self.chunks.len() - 1].pointer().end() as usize
    }

    fn serialize(mut self, root_chunk_index: usize, writer: &mut dyn Write) -> Result<usize> {
        let root_ptr = self
            .chunks
            .get(root_chunk_index)
            .ok_or_else(|| {
                Error::Consistency(format!("Root chunk with id {} not found", root_chunk_index))
            })?
            .pointer();
        let header = Header::new(self.byteswap, 0, root_ptr);
        self.chunks[0].data_mut()[0..size_of::<Header>()]
            .copy_from_slice(transmute_one_to_bytes(&header));

        let mut size = 0;
        for chunk in self.chunks.into_iter() {
            // Align
            if size < chunk.pointer().start() as usize {
                let padding = chunk.pointer().start() as usize - size;
                size += padding;
                writer.write_all(&vec![0; padding])?;
            }

            size += chunk.pointer().size();
            writer.write_all(&chunk.into_data())?;
        }

        Ok(size)
    }

    fn serialize_to_vec(self, root_chunk_index: usize) -> Result<Vec<u8>> {
        let mut vec = Vec::with_capacity(self.file_size());
        self.serialize(root_chunk_index, &mut vec)?;
        Ok(vec)
    }

    /// Write the GVDB file into the provided [`std::io::Write`]
    pub fn write_with_table(
        mut self,
        table_builder: HashTableBuilder,
        writer: &mut dyn Write,
    ) -> Result<usize> {
        let index = self.add_table_builder(table_builder)?.0;
        self.serialize(index, writer)
    }

    /// Create a [`Vec<u8>`] with the GVDB file data
    pub fn write_to_vec_with_table(mut self, table_builder: HashTableBuilder) -> Result<Vec<u8>> {
        let index = self.add_table_builder(table_builder)?.0;
        self.serialize_to_vec(index)
    }
}

impl Default for FileWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        read::{File, HashItemType},
        test::byte_compare_file_4,
    };
    use matches::assert_matches;
    use std::borrow::Cow;
    use std::io::Cursor;

    use crate::test::{
        assert_bytes_eq, assert_is_file_1, assert_is_file_2, byte_compare_file_1,
        byte_compare_file_2,
    };
    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};

    #[test]
    fn derives() {
        let ht_builder = HashTableBuilder::default();
        println!("{:?}", ht_builder);

        let chunk = Chunk::new(Box::new([0; 0]), Pointer::NULL);
        assert!(format!("{:?}", chunk).contains("Chunk"));
    }

    #[test]
    fn hash_table_builder1() {
        let mut builder = HashTableBuilder::new();
        assert!(builder.is_empty());
        builder.insert_string("string", "Test").unwrap();
        builder
            .insert_value("123", zvariant::Value::new(123u32))
            .unwrap();
        assert!(!builder.is_empty());
        assert_eq!(builder.len(), 2);

        let mut builder2 = HashTableBuilder::new();
        builder2.insert_bytes("bytes", &[1, 2, 3, 4]).unwrap();
        builder.insert_table("table", builder2).unwrap();

        let table = builder.build().unwrap();

        assert_eq!(
            table.get("string").unwrap().value_ref().value().unwrap(),
            &zvariant::Value::new("Test")
        );

        assert_eq!(
            table.get("123").unwrap().value_ref().value().unwrap(),
            &zvariant::Value::new(123u32)
        );

        let item = table.get("table").unwrap();
        assert_matches!(item.value_ref().table_builder(), Some(_));
        let val = item.value().take();
        assert_matches!(val, HashValue::TableBuilder(..));
        let HashValue::TableBuilder(tb) = val else {
            panic!("Invalid value");
        };

        let table2 = tb.build().unwrap();
        let data: &[u8] = &[1, 2, 3, 4];
        assert_eq!(
            table2.get("bytes").unwrap().value_ref().value().unwrap(),
            &zvariant::Value::new(data)
        );
    }

    #[test]
    fn hash_table_builder2() {
        let mut builder = HashTableBuilder::new();

        // invalid path
        builder.insert_string("string/", "collision").unwrap();
        let err = builder.insert_string("string/test", "test").unwrap_err();
        assert_matches!(err, Error::Consistency(_));

        let mut builder = HashTableBuilder::with_path_separator(None);
        // invalid path but this isn't important as path handling is turned off
        builder.insert_string("string/", "collision").unwrap();
        builder.insert_string("string/test", "test").unwrap();
    }

    #[test]
    fn file_builder_file_1() {
        let mut file_builder = FileWriter::new();
        let mut table_builder = HashTableBuilder::new();

        let value1 = 1234u32;
        let value2 = 98765u32;
        let value3 = "TEST_STRING_VALUE";
        let tuple_data = (value1, value2, value3);
        let variant = zvariant::Value::new(tuple_data);
        table_builder.insert_value("root_key", variant).unwrap();
        let root_index = file_builder.add_table_builder(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();

        println!("{:?}", root);

        assert_is_file_1(&root);
        byte_compare_file_1(&root);
    }

    #[test]
    fn file_builder_file_2() {
        let mut file_builder = FileWriter::for_big_endian();
        let mut table_builder = HashTableBuilder::new();

        table_builder
            .insert_string("string", "test string")
            .unwrap();

        let mut table_builder_2 = HashTableBuilder::new();
        table_builder_2.insert("int", 42u32).unwrap();

        table_builder
            .insert_table("table", table_builder_2)
            .unwrap();
        let root_index = file_builder.add_table_builder(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();

        println!("{:?}", root);

        assert_is_file_2(&root);
        byte_compare_file_2(&root);
    }

    #[test]
    fn file_builder_file_4() {
        let mut writer = FileWriter::new();
        let mut table_builder = HashTableBuilder::new();

        let mut dict = HashMap::<&str, zvariant::Value>::new();
        dict.insert("key1", "value1".into());
        dict.insert("key2", 2u32.into());
        let value = ("arg0", dict);

        table_builder.insert("struct", value).unwrap();
        let root_index = writer.add_table_builder(table_builder).unwrap().0;
        let bytes = writer.serialize_to_vec(root_index).unwrap();
        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();

        println!("{:?}", root);

        byte_compare_file_4(&root);
    }

    #[test]
    fn reproducible_build() {
        let mut last_data: Option<Vec<u8>> = None;

        for _ in 0..100 {
            let file_builder = FileWriter::new();
            let mut table_builder = HashTableBuilder::new();
            for num in 0..200 {
                let str = format!("{}", num);
                table_builder.insert_string(&str, &str).unwrap();
            }

            let data = file_builder.write_to_vec_with_table(table_builder).unwrap();
            if let Some(last_data) = last_data {
                assert_bytes_eq(&last_data, &data, "Reproducible builds");
            }

            last_data = Some(data);
        }
    }

    #[test]
    fn big_endian() {
        let mut file_builder = FileWriter::for_big_endian();
        let mut table_builder = HashTableBuilder::new();

        let value1 = 1234u32;
        let value2 = 98765u32;
        let value3 = "TEST_STRING_VALUE";
        let tuple_data = (value1, value2, value3);
        let variant = zvariant::Value::new(tuple_data);
        table_builder.insert_value("root_key", variant).unwrap();
        let root_index = file_builder.add_table_builder(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();

        // "GVariant" byteswapped at 32 bit boundaries is the header for big-endian GVariant files
        assert_eq!("raVGtnai", std::str::from_utf8(&bytes[0..8]).unwrap());

        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();
        println!("{:?}", root);

        assert_is_file_1(&root);
    }

    #[test]
    fn container() {
        let mut file_builder = FileWriter::new();
        let mut table_builder = HashTableBuilder::new();

        table_builder
            .insert_string("contained/string", "str")
            .unwrap();
        let root_index = file_builder.add_table_builder(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();

        let container_item = root
            .hash_table()
            .unwrap()
            .get_hash_item("contained/")
            .unwrap();

        assert_eq!(container_item.typ().unwrap(), HashItemType::Container);
        println!("{:?}", root);
    }

    #[test]
    fn missing_root() {
        let file = FileWriter::new();
        assert_matches!(file.serialize_to_vec(1), Err(Error::Consistency(_)));
    }

    #[test]
    fn missing_child() {
        let mut table = HashTableBuilder::new();
        let item = HashValue::Container(vec!["missing".to_string()]);
        table.insert_item_value("test", item).unwrap();

        assert_matches!(table.build(), Err(Error::Consistency(_)));
    }

    #[test]
    fn empty_key() {
        let mut table = HashTableBuilder::new();
        table.insert_string("", "test").unwrap();
        let file = FileWriter::new();
        let err = file.write_to_vec_with_table(table).unwrap_err();

        assert_matches!(err, Error::Consistency(_))
    }

    #[test]
    fn remove_child() {
        let mut table_builder = HashTableBuilder::new();
        table_builder.insert_string("test/test", "test").unwrap();
        table_builder.items.remove("test/test");
        let file = FileWriter::new();

        let err = file.write_to_vec_with_table(table_builder).unwrap_err();
        assert_matches!(err, Error::Consistency(_))
    }

    #[test]
    fn remove_child2() {
        let mut table_builder = HashTableBuilder::new();
        table_builder.insert_string("test/test", "test").unwrap();
        let mut table = table_builder.build().unwrap();
        table.remove("test/test");

        let mut file = FileWriter::new();
        let err = file.add_simple_hash_table(table).unwrap_err();
        assert_matches!(err, Error::Consistency(_))
    }

    #[test]
    fn io_error() {
        let file = FileWriter::default();

        // This buffer is intentionally too small to result in I/O error
        let buffer = [0u8; 10];
        let mut cursor = Cursor::new(buffer);
        let mut table = HashTableBuilder::new();
        table.insert("test", "test").unwrap();
        let err = file.write_with_table(table, &mut cursor).unwrap_err();
        assert_matches!(err, Error::Io(_, _));
        assert!(format!("{}", err).contains("I/O error"));
        assert!(format!("{:?}", err).contains("I/O error"));
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::File;
    use crate::test::{assert_gvariant_eq, byte_compare_file_4};
    use crate::write::hash::SimpleHashTable;
    use crate::write::item::HashValue;
    use crate::write::{FileWriter, HashTableBuilder};
    use glib::prelude::*;
    use std::borrow::Cow;

    #[test]
    fn simple_hash_table() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        let item = HashValue::GVariant("test".to_variant());
        table.insert("test", item);
        assert_eq!(table.n_items(), 1);
        assert_eq!(
            table.get("test").unwrap().value_ref().gvariant().unwrap(),
            &"test".to_variant()
        );
    }

    #[test]
    fn hash_table_builder() {
        let mut table = HashTableBuilder::new();
        table.insert_gvariant("test", "test".to_variant()).unwrap();
        let simple_ht = table.build().unwrap();
        assert_eq!(
            simple_ht
                .get("test")
                .unwrap()
                .value_ref()
                .gvariant()
                .unwrap(),
            &"test".to_variant()
        );
    }

    #[test]
    fn file_writer() {
        for byteswap in [true, false] {
            let mut table = HashTableBuilder::default();
            table.insert_gvariant("test", "test".to_variant()).unwrap();
            let writer = FileWriter::with_byteswap(byteswap);
            let _ = writer.write_to_vec_with_table(table).unwrap();
        }
    }

    #[test]
    fn file_builder_file_4_glib() {
        let mut writer = FileWriter::new();
        let mut table_builder = HashTableBuilder::new();

        let map = glib::VariantDict::new(None);
        map.insert("key1", "value1");
        map.insert("key2", 2u32);
        let value = ("arg0", map).to_variant();

        table_builder.insert_gvariant("struct", value).unwrap();
        let root_index = writer.add_table_builder(table_builder).unwrap().0;
        let bytes = writer.serialize_to_vec(root_index).unwrap();
        let root = File::from_bytes(Cow::Owned(bytes)).unwrap();

        println!("{:?}", root);

        byte_compare_file_4(&root);
    }

    #[test]
    /// Regression test for https://github.com/dbus2/zbus/issues/868
    fn gvariant_vs_zvariant() {
        let mut map_glib = std::collections::HashMap::<&str, &str>::new();
        map_glib.insert("k", "v");
        let variant_glib = glib::Variant::from_variant(&map_glib.to_variant()).normal_form();
        let data_glib = variant_glib.data();

        let mut map_zvariant = std::collections::HashMap::<&str, &str>::new();
        map_zvariant.insert("k", "v");
        let ctxt = zvariant::serialized::Context::new_gvariant(zvariant::LE, 0);

        let data_zvariant = zvariant::to_bytes(ctxt, &zvariant::Value::new(map_zvariant)).unwrap();

        assert_gvariant_eq(data_glib, &data_zvariant, "gvariant vs zvariant");
    }
}
