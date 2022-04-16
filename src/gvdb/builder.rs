use crate::gvdb::builder::GvdbBuilderItemValue::Container;
use crate::gvdb::error::{GvdbBuilderError, GvdbBuilderResult};
use crate::gvdb::hash::GvdbHashHeader;
use crate::gvdb::hash_item::{GvdbHashItem, GvdbHashItemType};
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use crate::gvdb::util::{align_offset, djb_hash};
use glib::ToVariant;
use safe_transmute::transmute_one_to_bytes;
use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::iter::Map;
use std::mem::size_of;
use std::rc::Rc;

#[derive(Debug)]
pub enum GvdbBuilderItemValue {
    Value(glib::Variant),
    TableBuilder(GvdbHashTableBuilder),

    // A child container with no additional value
    Container(Vec<String>),
}

impl Default for GvdbBuilderItemValue {
    fn default() -> Self {
        Self::Container(Vec::new())
    }
}

impl GvdbBuilderItemValue {
    pub fn typ(&self) -> GvdbHashItemType {
        match self {
            GvdbBuilderItemValue::Value(_) => GvdbHashItemType::Value,
            GvdbBuilderItemValue::TableBuilder(_) => GvdbHashItemType::HashTable,
            GvdbBuilderItemValue::Container(_) => GvdbHashItemType::Container,
        }
    }

    pub fn variant(&self) -> Option<&glib::Variant> {
        match self {
            GvdbBuilderItemValue::Value(variant) => Some(variant),
            _ => None,
        }
    }

    pub fn table_builder(&self) -> Option<&GvdbHashTableBuilder> {
        match self {
            GvdbBuilderItemValue::TableBuilder(tb) => Some(tb),
            _ => None,
        }
    }

    pub fn container(&self) -> Option<&Vec<String>> {
        match self {
            GvdbBuilderItemValue::Container(children) => Some(children),
            _ => None,
        }
    }
}

impl Into<GvdbBuilderItemValue> for glib::Variant {
    fn into(self) -> GvdbBuilderItemValue {
        GvdbBuilderItemValue::Value(self)
    }
}

impl Into<GvdbBuilderItemValue> for GvdbHashTableBuilder {
    fn into(self) -> GvdbBuilderItemValue {
        GvdbBuilderItemValue::TableBuilder(self)
    }
}

#[derive(Debug)]
pub struct GvdbBuilderItem {
    // The key string of the item
    key: String,

    // The djb hash
    hash: u32,

    // An arbitrary data container
    value: RefCell<GvdbBuilderItemValue>,

    // The assigned index for the gvdb file
    assigned_index: Cell<u32>,

    // The parent item of this builder item
    parent: RefCell<Option<Rc<GvdbBuilderItem>>>,

    // The next item in the hash bucket
    next: RefCell<Option<Rc<GvdbBuilderItem>>>,
}

impl GvdbBuilderItem {
    pub fn new(key: &str, hash: u32, value: GvdbBuilderItemValue) -> Self {
        let key = key.to_string();

        Self {
            key,
            hash,
            value: RefCell::new(value),
            assigned_index: Cell::new(u32::MAX),
            parent: Default::default(),
            next: Default::default(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn hash(&self) -> u32 {
        self.hash
    }

    pub fn value_ref(&self) -> Ref<GvdbBuilderItemValue> {
        self.value.borrow()
    }

    pub fn set_parent(&self, parent: &GvdbBuilderItem) -> GvdbBuilderResult<()> {
        if !self.key.starts_with(&parent.key) {
            return Err(GvdbBuilderError::WrongParentPrefix);
        }

        Ok(())
    }
}

pub struct SimpleHashTable {
    buckets: Vec<Option<Rc<GvdbBuilderItem>>>,
    n_items: usize,
}

impl SimpleHashTable {
    pub fn with_n_buckets(n_buckets: usize) -> Self {
        let mut buckets = Vec::with_capacity(n_buckets);
        buckets.resize_with(n_buckets, || None);

        Self {
            buckets,
            n_items: 0,
        }
    }

    pub fn n_buckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn n_items(&self) -> usize {
        self.n_items
    }

    fn hash_bucket(&self, hash_value: u32) -> usize {
        (hash_value % self.buckets.len() as u32) as usize
    }

    /// Insert the item for the specified key
    pub fn insert(&mut self, key: &str, item: GvdbBuilderItemValue) -> Rc<GvdbBuilderItem> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);

        let item = Rc::new(GvdbBuilderItem::new(key, hash_value, item));
        let replaced_item = std::mem::replace(&mut self.buckets[bucket], Some(item.clone()));
        if let Some(replaced_item) = replaced_item {
            if replaced_item.key == key {
                // Replace
                self.buckets[bucket]
                    .as_ref()
                    .unwrap()
                    .next
                    .replace(replaced_item.next.take());
            } else {
                // Insert
                self.buckets[bucket]
                    .as_ref()
                    .unwrap()
                    .next
                    .replace(Some(replaced_item));
                self.n_items += 1;
            }
        } else {
            // Insert to empty bucket
            self.n_items += 1;
        }

        item
    }

    /// Remove the item with the specified key
    pub fn remove(&mut self, key: &str) {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);

        // Remove the item if it already exists
        if let Some((previous, item)) = self.get_from_bucket(key, bucket) {
            if let Some(previous) = previous {
                previous.next.replace(item.next.take());
            } else {
                self.buckets[bucket] = item.next.take();
            }

            self.n_items -= 1;
        }
    }

    fn get_from_bucket(
        &self,
        key: &str,
        bucket: usize,
    ) -> Option<(Option<Rc<GvdbBuilderItem>>, Rc<GvdbBuilderItem>)> {
        let mut item = self.buckets.get(bucket)?.clone();
        let mut previous = None;

        while let Some(current_item) = item {
            if current_item.key == key {
                return Some((previous, current_item.clone()));
            } else {
                previous = Some(current_item.clone());
                item = current_item.next.borrow().clone();
            }
        }

        None
    }

    pub fn get(&self, key: &str) -> Option<Rc<GvdbBuilderItem>> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);
        self.get_from_bucket(key, bucket).map(|r| r.1)
    }

    pub fn into_buckets(self) -> Vec<Option<Rc<GvdbBuilderItem>>> {
        self.buckets
    }

    pub fn iter(&self) -> SimpleHashTableIter<'_> {
        SimpleHashTableIter {
            hash_table: self,
            bucket: 0,
            last_item: None,
        }
    }

    pub fn items_iter(
        &self,
    ) -> Map<SimpleHashTableIter<'_>, fn((usize, Rc<GvdbBuilderItem>)) -> Rc<GvdbBuilderItem>> {
        self.iter().map(|(_bucket, item)| item)
    }
}

pub struct SimpleHashTableIter<'a> {
    hash_table: &'a SimpleHashTable,
    bucket: usize,
    last_item: Option<Rc<GvdbBuilderItem>>,
}

impl<'a> Iterator for SimpleHashTableIter<'a> {
    type Item = (usize, Rc<GvdbBuilderItem>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(last_item) = self.last_item.clone() {
            // First check if there are more items in this bucket
            if let Some(next_item) = &*last_item.next.borrow() {
                // Next item in the same bucket
                self.last_item = Some(next_item.clone());
                return Some((self.bucket, next_item.clone()));
            } else {
                // Last item in the bucket, check the next bucket
                self.bucket += 1;
            }
        }

        while let Some(bucket_item) = self.hash_table.buckets.get(self.bucket) {
            self.last_item = None;

            // This bucket might be empty
            if let Some(item) = bucket_item {
                // We found something
                self.last_item = Some(item.clone());
                return Some((self.bucket, item.clone()));
            } else {
                // Empty bucket, continue with next bucket
                self.bucket += 1;
            }
        }

        // Nothing left
        None
    }
}

impl<'a> IntoIterator for &'a SimpleHashTable {
    type Item = (usize, Rc<GvdbBuilderItem>);
    type IntoIter = SimpleHashTableIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub struct GvdbHashTableBuilder {
    items: HashMap<String, GvdbBuilderItemValue>,
    path_separator: Option<String>,
}

impl GvdbHashTableBuilder {
    pub fn new() -> Self {
        Self::with_path_separator(Some("/"))
    }

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
            let mut iter = key.split(sep).into_iter().peekable();
            let mut last_key: Option<String> = None;

            while let Some(segment) = iter.next() {
                this_key += segment;
                if this_key != key {
                    this_key += sep;
                }

                if let Some(last_key) = last_key {
                    if let Some(last_item) = self.items.get_mut(&last_key) {
                        if let Container(ref mut container) = last_item {
                            if !container.contains(&this_key) {
                                container.push(this_key.clone());
                            }
                        } else {
                            return Err(GvdbBuilderError::Consistency(format!(
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
            self.items.insert(key.into(), item);
        }

        Ok(())
    }

    pub fn insert_variant<S: Into<String>>(
        &mut self,
        key: S,
        variant: glib::Variant,
    ) -> GvdbBuilderResult<()> {
        let item = GvdbBuilderItemValue::Value(variant);
        self.insert(key, item)
    }

    pub fn insert_string<S: Into<String>>(&mut self, key: S, value: &str) -> GvdbBuilderResult<()> {
        let variant = value.to_variant();
        self.insert_variant(key, variant)
    }

    pub fn insert_bytes<S: Into<String>>(&mut self, key: S, bytes: &[u8]) -> GvdbBuilderResult<()> {
        let bytes = glib::Bytes::from(bytes);
        let variant = glib::Variant::from_bytes_with_type(&bytes, glib::VariantTy::BYTE_STRING);
        self.insert_variant(key, variant)
    }

    pub fn insert_table<S: Into<String>>(
        &mut self,
        key: S,
        table_builder: GvdbHashTableBuilder,
    ) -> GvdbBuilderResult<()> {
        let item = GvdbBuilderItemValue::TableBuilder(table_builder);
        self.insert(key, item)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn build(self) -> GvdbBuilderResult<SimpleHashTable> {
        let mut hash_table = SimpleHashTable::with_n_buckets(self.items.len());

        for (key, value) in self.items.into_iter() {
            hash_table.insert(&key, value);
        }

        for (key, item) in &hash_table {
            if let Some(container) = item.value.borrow().container() {
                for child in container {
                    let child_item = hash_table.get(child);
                    if let Some(child_item) = child_item {
                        child_item.parent.replace(Some(item.clone()));
                    } else {
                        return Err(GvdbBuilderError::Consistency(format!("Tried to set parent for child '{}' to '{}' but the child was not found.", child, key)));
                    }
                }
            }
        }

        Ok(hash_table)
    }
}

pub enum GvdbItemValue {
    Value(glib::Variant),
    Table(SimpleHashTable),
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

    pub fn pointer(&self) -> &GvdbPointer {
        &self.pointer
    }
}

pub struct GvdbFileWriter {
    offset: usize,
    chunks: VecDeque<GvdbChunk>,
    byteswap: bool,
}

impl GvdbFileWriter {
    pub fn new(byteswap: bool) -> Self {
        let mut this = Self {
            offset: 0,
            chunks: Default::default(),
            byteswap,
        };

        this.allocate_empty_chunk(size_of::<GvdbHeader>(), 1);
        this
    }

    /// Allocate a chunk and return its index
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

    fn chunk_mut(&mut self, index: usize) -> Option<&mut GvdbChunk> {
        self.chunks.get_mut(index)
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

        for (index, item) in table.items_iter().enumerate() {
            item.assigned_index.set(index as u32);
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

            let parent = if let Some(parent) = &*current_item.parent.borrow() {
                parent.assigned_index.get()
            } else {
                u32::MAX
            };

            let key = if let Some(parent) = &*current_item.parent.borrow() {
                current_item.key().strip_prefix(&parent.key).unwrap_or("")
            } else {
                current_item.key()
            };

            if key.is_empty() {
                return Err(GvdbBuilderError::EmptyKey);
            }

            let key_ptr = self.add_string(key).1.pointer;
            let typ = current_item.value_ref().typ();

            let value_ptr = match current_item.value.take() {
                GvdbBuilderItemValue::Value(value) => self.add_variant(&value).1.pointer,
                GvdbBuilderItemValue::TableBuilder(tb) => self.add_hash_table(tb)?.1.pointer,
                GvdbBuilderItemValue::Container(children) => {
                    let size = children.len() * size_of::<u32>();
                    let chunk = self.allocate_empty_chunk(size, 4).1;

                    let mut offset = 0;
                    for child in children {
                        let child_item = table.get(&child);
                        if let Some(child_item) = child_item {
                            child_item.parent.replace(Some(current_item.clone()));

                            chunk.data_mut()[offset..offset + size_of::<u32>()].copy_from_slice(
                                &u32::to_le_bytes(child_item.assigned_index.get()),
                            );
                            offset += size_of::<u32>();
                        } else {
                            return Err(GvdbBuilderError::Consistency(format!(
                                "Child item '{}' not found for parent: '{}'",
                                child, key
                            )));
                        }
                    }

                    chunk.pointer
                }
            };

            let hash_item = GvdbHashItem::new(current_item.hash, parent, key_ptr, typ, value_ptr);

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
        self.chunks[self.chunks.len() - 1].pointer.end() as usize
    }

    fn serialize(
        mut self,
        root_chunk_index: usize,
        writer: &mut dyn Write,
    ) -> GvdbBuilderResult<usize> {
        let root_ptr = self
            .chunks
            .get(root_chunk_index)
            .ok_or(GvdbBuilderError::InvalidRootChunk)?
            .pointer;
        let header = GvdbHeader::new(self.byteswap, 0, root_ptr);
        self.chunks[0].data_mut()[0..size_of::<GvdbHeader>()]
            .copy_from_slice(transmute_one_to_bytes(&header));

        let mut size = 0;
        for chunk in self.chunks.into_iter() {
            // Align
            if size < chunk.pointer.start() as usize {
                let padding = chunk.pointer.start() as usize - size;
                size += padding;
                writer.write_all(&vec![0; padding])?;
            }

            size += chunk.pointer.size() as usize;
            writer.write_all(&chunk.into_data())?;
        }

        Ok(size)
    }

    fn serialize_to_vec(self, root_chunk_index: usize) -> GvdbBuilderResult<Vec<u8>> {
        let mut vec = Vec::with_capacity(self.file_size());
        self.serialize(root_chunk_index, &mut vec)?;
        Ok(vec)
    }

    pub fn write_with_table(
        mut self,
        table_builder: GvdbHashTableBuilder,
        writer: &mut dyn Write,
    ) -> GvdbBuilderResult<usize> {
        let index = self.add_hash_table(table_builder)?.0;
        self.serialize(index, writer)
    }

    pub fn write_into_vec_with_table(
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
    use crate::gvdb::file::test::*;
    use crate::gvdb::file::GvdbFile;
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
        assert_eq!(table.n_items, 1);
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

        assert_eq!(table.n_items, 20);

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
        let val = item.value.take();
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
        let mut file_builder = GvdbFileWriter::new(false);
        let mut table_builder = GvdbHashTableBuilder::new();

        let value1 = 1234u32.to_variant();
        let value2 = 98765u32.to_variant();
        let value3 = "TEST_STRING_VALUE".to_variant();
        let tuple_data = vec![value1, value2, value3];
        let variant = glib::Variant::tuple_from_iter(&tuple_data);
        table_builder.insert_variant("root_key", variant).unwrap();
        let root_index = file_builder.add_hash_table(table_builder).unwrap().0;
        let bytes = file_builder.serialize_to_vec(root_index).unwrap();
        let root = GvdbFile::from_bytes(Cow::Owned(bytes), true).unwrap();
        assert_is_file_1(&root);
        byte_compare_file_1(&root);
    }

    #[test]
    pub fn file_builder_file_2() {
        let mut file_builder = GvdbFileWriter::new(false);
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
        let root = GvdbFile::from_bytes(Cow::Owned(bytes), true).unwrap();
        assert_is_file_2(&root);
        byte_compare_file_2(&root);
    }
}
