use crate::read::hash::GvdbHashHeader;
use crate::read::hash_item::GvdbHashItem;
use crate::read::header::GvdbHeader;
use crate::read::pointer::GvdbPointer;
use crate::util::align_offset;
use crate::write::error::{GvdbBuilderResult, GvdbWriterError};
use crate::write::hash::SimpleHashTable;
use crate::write::item::GvdbBuilderItemValue;
use glib::ToVariant;
use safe_transmute::transmute_one_to_bytes;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::mem::size_of;

/// Create hash tables for use in GVDB files
///
/// # Example
///
/// ```
/// use glib::ToVariant;
/// use gvdb::write::file::{GvdbFileWriter, GvdbHashTableBuilder};
///
/// let file_writer = GvdbFileWriter::new();
/// let mut table_builder = GvdbHashTableBuilder::new();
/// table_builder
///     .insert_string("string", "test string")
///     .unwrap();
/// let gvdb_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
/// ```
#[derive(Debug)]
pub struct GvdbHashTableBuilder {
    items: HashMap<String, GvdbBuilderItemValue>,
    path_separator: Option<String>,
}

impl GvdbHashTableBuilder {
    /// Create a new empty GvdbHashTableBuilder with the default path separator `/`
    ///
    /// ```
    /// # use gvdb::write::file::GvdbHashTableBuilder;
    /// let mut table_builder = GvdbHashTableBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::with_path_separator(Some("/"))
    }

    /// Create a new empty GvdbHashTableBuilder a different path separator than `/` or none at all
    ///
    /// ```
    /// # use gvdb::write::file::GvdbHashTableBuilder;
    /// let mut table_builder = GvdbHashTableBuilder::with_path_separator(Some(":"));
    /// ```
    pub fn with_path_separator(sep: Option<&str>) -> Self {
        Self {
            items: Default::default(),
            path_separator: sep.map(|s| s.to_string()),
        }
    }

    fn insert<S: Into<String>>(
        &mut self,
        key: S,
        item: GvdbBuilderItemValue,
    ) -> GvdbBuilderResult<()> {
        let key = key.into();

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
                        if let GvdbBuilderItemValue::Container(ref mut container) = last_item {
                            if !container.contains(&this_key) {
                                container.push(this_key.clone());
                            }
                        } else {
                            return Err(GvdbWriterError::Consistency(format!(
                                "Parent item with key '{}' is not of type container",
                                this_key
                            )));
                        }
                    } else {
                        let parent_item = GvdbBuilderItemValue::Container(vec![this_key.clone()]);
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

    /// Insert GVariant `item` for `key`
    ///
    /// ```
    /// # use glib::ToVariant;
    /// # let mut table_builder = gvdb::write::file::GvdbHashTableBuilder::new();
    /// let variant = 123u32.to_variant();
    /// table_builder.insert_variant("variant_123", variant);
    /// ```
    pub fn insert_variant<S: Into<String>>(
        &mut self,
        key: S,
        variant: glib::Variant,
    ) -> GvdbBuilderResult<()> {
        let item = GvdbBuilderItemValue::Value(variant);
        self.insert(key, item)
    }

    /// Convenience method to create a string type GVariant for `value` and insert it at `key`
    ///
    /// ```
    /// # let mut table_builder = gvdb::write::file::GvdbHashTableBuilder::new();
    /// table_builder.insert_string("string_key", "string_data");
    /// ```
    pub fn insert_string<S: Into<String>>(&mut self, key: S, value: &str) -> GvdbBuilderResult<()> {
        let variant = value.to_variant();
        self.insert_variant(key, variant)
    }

    /// Convenience method to create a byte type GVariant for `value` and insert it at `key`
    ///
    /// ```
    /// # let mut table_builder = gvdb::write::file::GvdbHashTableBuilder::new();
    /// table_builder.insert_bytes("bytes", &[1, 2, 3, 4, 5]);
    /// ```
    pub fn insert_bytes<S: Into<String>>(&mut self, key: S, bytes: &[u8]) -> GvdbBuilderResult<()> {
        let bytes = glib::Bytes::from(bytes);
        let variant = glib::Variant::from_bytes_with_type(&bytes, glib::VariantTy::BYTE_STRING);
        self.insert_variant(key, variant)
    }

    /// Insert an entire hash table at `key`.
    ///
    /// ```
    /// # use glib::ToVariant;
    /// # use gvdb::write::file::GvdbHashTableBuilder;
    /// let mut table_builder = GvdbHashTableBuilder::new();
    /// let mut table_builder_2 = GvdbHashTableBuilder::new();
    /// table_builder_2
    ///     .insert_variant("int", 42u32.to_variant())
    ///     .unwrap();
    ///
    /// table_builder
    ///     .insert_table("table", table_builder_2)
    ///     .unwrap();
    /// ```
    pub fn insert_table<S: Into<String>>(
        &mut self,
        key: S,
        table_builder: GvdbHashTableBuilder,
    ) -> GvdbBuilderResult<()> {
        let item = GvdbBuilderItemValue::TableBuilder(table_builder);
        self.insert(key, item)
    }

    /// The number of items contained in the hash table builder
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the hash table builder contains no items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn build(self) -> GvdbBuilderResult<SimpleHashTable> {
        let mut hash_table = SimpleHashTable::with_n_buckets(self.items.len());

        for (key, value) in self.items.into_iter() {
            hash_table.insert(&key, value);
        }

        for (key, item) in &hash_table {
            if let Some(container) = item.value_ref().container() {
                for child in container {
                    let child_item = hash_table.get(child);
                    if let Some(child_item) = child_item {
                        child_item.parent().replace(Some(item.clone()));
                    } else {
                        return Err(GvdbWriterError::Consistency(format!("Tried to set parent for child '{}' to '{}' but the child was not found.", child, key)));
                    }
                }
            }
        }

        Ok(hash_table)
    }
}

impl Default for GvdbHashTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

struct GvdbChunk {
    // The pointer that points to the data where the chunk will be in memory in the finished file
    pointer: GvdbPointer,

    // We use a boxed slice because this guarantees that the size is not changed afterwards
    data: Box<[u8]>,
}

impl GvdbChunk {
    pub fn new(data: Box<[u8]>, pointer: GvdbPointer) -> Self {
        Self { pointer, data }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Box<[u8]> {
        self.data
    }

    pub fn pointer(&self) -> GvdbPointer {
        self.pointer
    }
}

/// Create GVDB files
///
/// # Example
/// ```
/// use glib::ToVariant;
/// use gvdb::write::file::{GvdbFileWriter, GvdbHashTableBuilder};
///
/// fn create_gvdb_file() {
///     let mut file_writer = GvdbFileWriter::new();
///     let mut table_builder = GvdbHashTableBuilder::new();
///     table_builder
///            .insert_string("string", "test string")
///            .unwrap();
///     let file_data = file_writer.write_to_vec_with_table(table_builder).unwrap();
/// }
/// ```
pub struct GvdbFileWriter {
    offset: usize,
    chunks: VecDeque<GvdbChunk>,
    byteswap: bool,
}

impl GvdbFileWriter {
    /// Create a new instance configured for writing little endian data (preferred endianness)
    /// ```
    /// let file_writer = gvdb::write::file::GvdbFileWriter::new();
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
    /// let file_writer = gvdb::write::file::GvdbFileWriter::new();
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

        this.allocate_empty_chunk(size_of::<GvdbHeader>(), 1);
        this
    }

    /// Allocate a chunk
    fn allocate_chunk_with_data(
        &mut self,
        data: Box<[u8]>,
        alignment: usize,
    ) -> (usize, &mut GvdbChunk) {
        // Align the data
        self.offset = align_offset(self.offset, alignment);

        // Calculate the pointer
        let offset_start = self.offset;
        let offset_end = offset_start + data.len();
        let pointer = GvdbPointer::new(offset_start, offset_end);

        // Update the offset to the end of the chunk
        self.offset = offset_end;

        let chunk = GvdbChunk::new(data, pointer);
        self.chunks.push_back(chunk);
        let index = self.chunks.len() - 1;
        (index, &mut self.chunks[index])
    }

    fn allocate_empty_chunk(&mut self, size: usize, alignment: usize) -> (usize, &mut GvdbChunk) {
        let data = vec![0; size].into_boxed_slice();
        self.allocate_chunk_with_data(data, alignment)
    }

    fn add_variant(&mut self, variant: &glib::Variant) -> (usize, &mut GvdbChunk) {
        let value = if self.byteswap {
            glib::Variant::from_variant(&variant.byteswap())
        } else {
            glib::Variant::from_variant(variant)
        };

        let normal = glib::Variant::normal_form(&value);
        let data = normal.data();
        self.allocate_chunk_with_data(data.to_vec().into_boxed_slice(), 8)
    }

    fn add_string(&mut self, string: &str) -> (usize, &mut GvdbChunk) {
        let data = string.to_string().into_boxed_str().into_boxed_bytes();
        self.allocate_chunk_with_data(data, 1)
    }

    fn add_hash_table(
        &mut self,
        table_builder: GvdbHashTableBuilder,
    ) -> GvdbBuilderResult<(usize, &mut GvdbChunk)> {
        let table = table_builder.build()?;

        for (index, (_bucket, item)) in table.iter().enumerate() {
            item.set_assigned_index(index as u32);
        }

        let header = GvdbHashHeader::new(5, 0, table.n_buckets() as u32);
        let items_len = table.n_items() * size_of::<GvdbHashItem>();
        let size = size_of::<GvdbHashHeader>()
            + header.bloom_words_len()
            + header.buckets_len()
            + items_len;

        let hash_buckets_offset = size_of::<GvdbHashHeader>() + header.bloom_words_len();
        let hash_items_offset = hash_buckets_offset + header.buckets_len();

        let (hash_table_chunk_index, hash_table_chunk) = self.allocate_empty_chunk(size, 4);
        let header = transmute_one_to_bytes(&header);
        hash_table_chunk.data_mut()[0..header.len()].copy_from_slice(header);

        let mut current_bucket = usize::MAX;
        for (n_item, (n_bucket, current_item)) in table.iter().enumerate() {
            while current_bucket != n_bucket {
                current_bucket = current_bucket.wrapping_add(1);
                let hash_bucket_start = hash_buckets_offset + current_bucket * size_of::<u32>();
                let hash_bucket_end = hash_bucket_start + size_of::<u32>();

                self.chunks[hash_table_chunk_index].data[hash_bucket_start..hash_bucket_end]
                    .copy_from_slice(u32::to_le_bytes(n_item as u32).as_slice());
            }

            let parent = if let Some(parent) = &*current_item.parent_ref() {
                parent.assigned_index()
            } else {
                u32::MAX
            };

            let key = if let Some(parent) = &*current_item.parent_ref() {
                current_item.key().strip_prefix(&parent.key()).unwrap_or("")
            } else {
                current_item.key()
            };

            if key.is_empty() {
                return Err(GvdbWriterError::Consistency(format!(
                    "Item '{}' already exists in hash map",
                    current_item.key()
                )));
            }

            let key_ptr = self.add_string(key).1.pointer();
            let typ = current_item.value_ref().typ();

            let value_ptr = match current_item.value().take() {
                GvdbBuilderItemValue::Value(value) => self.add_variant(&value).1.pointer(),
                GvdbBuilderItemValue::TableBuilder(tb) => self.add_hash_table(tb)?.1.pointer(),
                GvdbBuilderItemValue::Container(children) => {
                    let size = children.len() * size_of::<u32>();
                    let chunk = self.allocate_empty_chunk(size, 4).1;

                    let mut offset = 0;
                    for child in children {
                        let child_item = table.get(&child);
                        if let Some(child_item) = child_item {
                            child_item.parent().replace(Some(current_item.clone()));

                            chunk.data_mut()[offset..offset + size_of::<u32>()]
                                .copy_from_slice(&u32::to_le_bytes(child_item.assigned_index()));
                            offset += size_of::<u32>();
                        } else {
                            return Err(GvdbWriterError::Consistency(format!(
                                "Child item '{}' not found for parent: '{}'",
                                child, key
                            )));
                        }
                    }

                    chunk.pointer()
                }
            };

            let hash_item = GvdbHashItem::new(current_item.hash(), parent, key_ptr, typ, value_ptr);

            let hash_item_start = hash_items_offset + n_item * size_of::<GvdbHashItem>();
            let hash_item_end = hash_item_start + size_of::<GvdbHashItem>();

            self.chunks[hash_table_chunk_index].data[hash_item_start..hash_item_end]
                .copy_from_slice(transmute_one_to_bytes(&hash_item));
        }

        Ok((
            hash_table_chunk_index,
            &mut self.chunks[hash_table_chunk_index],
        ))
    }

    fn file_size(&self) -> usize {
        self.chunks[self.chunks.len() - 1].pointer().end() as usize
    }

    fn serialize(
        mut self,
        root_chunk_index: usize,
        writer: &mut dyn Write,
    ) -> GvdbBuilderResult<usize> {
        let root_ptr = self
            .chunks
            .get(root_chunk_index)
            .ok_or_else(|| {
                GvdbWriterError::Consistency(format!(
                    "Root chunk with id {} not found",
                    root_chunk_index
                ))
            })?
            .pointer();
        let header = GvdbHeader::new(self.byteswap, 0, root_ptr);
        self.chunks[0].data_mut()[0..size_of::<GvdbHeader>()]
            .copy_from_slice(transmute_one_to_bytes(&header));

        let mut size = 0;
        for chunk in self.chunks.into_iter() {
            // Align
            if size < chunk.pointer().start() as usize {
                let padding = chunk.pointer().start() as usize - size;
                size += padding;
                writer.write_all(&vec![0; padding])?;
            }

            size += chunk.pointer().size() as usize;
            writer.write_all(&chunk.into_data())?;
        }

        Ok(size)
    }

    fn serialize_to_vec(self, root_chunk_index: usize) -> GvdbBuilderResult<Vec<u8>> {
        let mut vec = Vec::with_capacity(self.file_size());
        self.serialize(root_chunk_index, &mut vec)?;
        Ok(vec)
    }

    /// Write the GVDB file into the provided [`std::io::Write`]
    pub fn write_with_table(
        mut self,
        table_builder: GvdbHashTableBuilder,
        writer: &mut dyn Write,
    ) -> GvdbBuilderResult<usize> {
        let index = self.add_hash_table(table_builder)?.0;
        self.serialize(index, writer)
    }

    /// Create a [`Vec<u8>`] with the GVDB file data
    pub fn write_to_vec_with_table(
        mut self,
        table_builder: GvdbHashTableBuilder,
    ) -> GvdbBuilderResult<Vec<u8>> {
        let index = self.add_hash_table(table_builder)?.0;
        self.serialize_to_vec(index)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::read::file::test::*;
    use crate::read::file::GvdbFile;
    use glib::{Bytes, ToVariant};
    use matches::assert_matches;
    use std::borrow::Cow;

    #[allow(unused_imports)]
    use pretty_assertions::{assert_eq, assert_ne, assert_str_eq};

    #[test]
    fn simple_hash_table() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        let item = GvdbBuilderItemValue::Value("test".to_variant());
        table.insert("test", item);
        assert_eq!(table.n_items(), 1);
        assert_eq!(
            table.get("test").unwrap().value_ref().variant().unwrap(),
            &"test".to_variant()
        );
    }

    #[test]
    fn simple_hash_table_2() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        for index in 0..20 {
            table.insert(&format!("{}", index), index.to_variant().into());
        }

        assert_eq!(table.n_items(), 20);

        for index in 0..20 {
            assert_eq!(
                index.to_variant(),
                *table
                    .get(&format!("{}", index))
                    .unwrap()
                    .value_ref()
                    .variant()
                    .unwrap()
            );
        }

        for index in 0..10 {
            let index = index * 2;
            table.remove(&format!("{}", index));
        }

        for index in 0..20 {
            let item = table.get(&format!("{}", index));
            assert_eq!(index % 2 == 1, item.is_some());
        }
    }

    #[test]
    pub fn gvdb_hash_table_builder() {
        let mut builder = GvdbHashTableBuilder::new();
        builder.insert_string("string", "Test").unwrap();
        builder.insert_variant("123", 123u32.to_variant()).unwrap();

        let mut builder2 = GvdbHashTableBuilder::new();
        builder2.insert_bytes("bytes", &[1, 2, 3, 4]).unwrap();
        builder.insert_table("table", builder2).unwrap();

        let table = builder.build().unwrap();

        assert_eq!(
            table.get("string").unwrap().value_ref().variant().unwrap(),
            &"Test".to_variant()
        );

        assert_eq!(
            table.get("123").unwrap().value_ref().variant().unwrap(),
            &123u32.to_variant()
        );

        let item = table.get("table").unwrap();
        assert_matches!(item.value_ref().table_builder(), Some(_));
        let val = item.value().take();
        assert_matches!(val, GvdbBuilderItemValue::TableBuilder(..));
        let tb = if let GvdbBuilderItemValue::TableBuilder(tb) = val {
            tb
        } else {
            panic!("Invalid value");
        };

        let table2 = tb.build().unwrap();
        assert_eq!(
            table2.get("bytes").unwrap().value_ref().variant().unwrap(),
            &Bytes::from(&[1, 2, 3, 4]).to_variant()
        );
    }

    #[test]
    pub fn file_builder_file_1() {
        let mut file_builder = GvdbFileWriter::new();
        let mut table_builder = GvdbHashTableBuilder::new();

        let value1 = 1234u32.to_variant();
        let value2 = 98765u32.to_variant();
        let value3 = "TEST_STRING_VALUE".to_variant();
        let tuple_data = vec![value1, value2, value3];
        let variant = glib::Variant::tuple_from_iter(&tuple_data);
        table_builder.insert_variant("root_key", variant).unwrap();
        let root_index = file_builder.add_hash_table(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(bytes)).unwrap();
        assert_is_file_1(&root);
        byte_compare_file_1(&root);
    }

    #[test]
    pub fn file_builder_file_2() {
        let mut file_builder = GvdbFileWriter::new();
        let mut table_builder = GvdbHashTableBuilder::new();

        table_builder
            .insert_string("string", "test string")
            .unwrap();

        let mut table_builder_2 = GvdbHashTableBuilder::new();
        table_builder_2
            .insert_variant("int", 42u32.to_variant())
            .unwrap();

        table_builder
            .insert_table("table", table_builder_2)
            .unwrap();
        let root_index = file_builder.add_hash_table(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(bytes)).unwrap();
        assert_is_file_2(&root);
        byte_compare_file_2(&root);
    }
}
