use crate::gvdb::error::GvdbResult;
use crate::gvdb::hash::GvdbHashHeader;
use crate::gvdb::hash_item::GvdbHashItem;
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use crate::gvdb::util::{align_offset, djb_hash};
use safe_transmute::transmute_one_to_bytes;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem::size_of;
use std::rc::Rc;
use std::str::FromStr;

pub struct Link<T>(Rc<RefCell<T>>);
pub type OptLink<T> = Option<Link<T>>;

impl<T> Link<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn borrow(&self) -> std::cell::Ref<T> {
        (*self.0).borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<T> {
        (*self.0).borrow_mut()
    }
}

impl<T> Clone for Link<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub enum GvdbBuilderValue {
    Value(glib::Variant),
    TableBuilder(GvdbHashTableBuilder),
}

pub struct SimpleHashTableItem<T> {
    key: String,
    hash: u32,
    value: T,
    next: Option<Box<SimpleHashTableItem<T>>>,
    parent: Option<String>,
    children: Vec<String>,
}

impl<T> SimpleHashTableItem<T> {
    pub fn new(key: &str, hash: u32, value: T) -> Self {
        let key = key.to_string();

        Self {
            key,
            hash,
            value,
            next: None,
            parent: None,
            children: vec![],
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn hash(&self) -> u32 {
        self.hash
    }

    pub fn value_ref(&self) -> &T {
        &self.value
    }

    pub fn into_next(self) -> Option<Box<Self>> {
        self.next
    }

    pub fn parent(&self) -> &Option<String> {
        &self.parent
    }

    pub fn set_parent_key(&mut self, parent: Option<String>) {
        self.parent = parent;
    }

    pub fn children_keys(&self) -> &[String] {
        &self.children
    }

    pub fn add_child_key(&mut self, child_key: String) {
        self.children.push(child_key);
    }
}

pub struct SimpleHashTable<T> {
    buckets: Vec<Option<Box<SimpleHashTableItem<T>>>>,
    n_items: usize,
}

impl<T> SimpleHashTable<T> {
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
    pub fn insert(&mut self, key: &str, item: T) {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);

        let item = SimpleHashTableItem::new(key, hash_value, item);
        let replaced_item = std::mem::replace(&mut self.buckets[bucket], Some(Box::new(item)));
        if let Some(replaced_item) = replaced_item {
            if replaced_item.key == key {
                // Replace
                self.buckets[bucket].as_mut().unwrap().next = replaced_item.next;
            } else {
                // Insert
                self.buckets[bucket].as_mut().unwrap().next = Some(replaced_item);
                self.n_items += 1;
            }
        } else {
            // Insert to empty bucket
            self.n_items += 1;
        }
    }

    /// Remove the item with the specified key
    pub fn remove(&mut self, key: &str) {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);

        // Remove the item if it already exists
        if let Some((item, previous)) = self.get_from_bucket_mut(key, bucket) {
            if previous {
                let previous_item = item;
                if let Some(mut item) = previous_item.next.take() {
                    previous_item.next = item.next.take();
                }
            } else {
                self.buckets[bucket] = item.next.take();
            }

            self.n_items -= 1;
        }
    }

    fn get_from_bucket_mut(
        &mut self,
        key: &str,
        bucket: usize,
    ) -> Option<(&mut Box<SimpleHashTableItem<T>>, bool)> {
        let item = self.buckets.get_mut(bucket)?;

        if let Some(item) = item {
            let mut item = item;

            loop {
                if item.next.is_some() && item.next.as_ref().unwrap().key == key {
                    return Some((item, true));
                } else {
                    if item.key == key {
                        return Some((item, false));
                    } else if item.next.is_some() {
                        item = item.next.as_mut().unwrap();
                    } else {
                        return None;
                    }
                }
            }
        } else {
            None
        }
    }

    fn get_from_bucket(
        &self,
        key: &str,
        bucket: usize,
    ) -> Option<(&Box<SimpleHashTableItem<T>>, bool)> {
        let item = self.buckets.get(bucket)?;

        if let Some(item) = item {
            let mut item = item;

            loop {
                if item.next.is_some() && item.next.as_ref().unwrap().key == key {
                    return Some((item, true));
                } else {
                    if item.key == key {
                        return Some((item, false));
                    } else if item.next.is_some() {
                        item = item.next.as_ref().unwrap();
                    } else {
                        return None;
                    }
                }
            }
        } else {
            None
        }
    }

    pub fn get_item(&self, key: &str) -> Option<&Box<SimpleHashTableItem<T>>> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);
        self.get_from_bucket(key, bucket)
            .and_then(|(item, previous)| {
                if previous {
                    item.next.as_ref()
                } else {
                    Some(item)
                }
            })
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.get_item(key).map(|x| x.value_ref())
    }

    pub fn get_child_items(&self, key: &str) -> Vec<&Box<SimpleHashTableItem<T>>> {
        let item = self.get_item(key);
        if let Some(item) = item {
            let children_keys = item.children_keys();
            let mut children = Vec::with_capacity(children_keys.len());

            for key in children_keys {
                let child = self.get_item(key);
                if let Some(child) = child {
                    children.push(child);
                }
            }

            children
        } else {
            vec![]
        }
    }

    pub fn get_parent_item(&self, key: &str) -> Option<&Box<SimpleHashTableItem<T>>> {
        let item = self.get_item(key);
        if let Some(item) = item {
            if let Some(parent) = &item.parent {
                self.get_item(parent)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn into_buckets(self) -> Vec<Option<Box<SimpleHashTableItem<T>>>> {
        self.buckets
    }
}

pub struct GvdbHashTableBuilder {
    items: Vec<(String, GvdbBuilderValue)>,
}

impl GvdbHashTableBuilder {
    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    fn insert(&mut self, key: String, item: GvdbBuilderValue) {
        self.items.push((key, item));
    }

    pub fn insert_variant(&mut self, key: String, variant: glib::Variant) {
        let item = GvdbBuilderValue::Value(variant);
        self.insert(key, item);
    }

    pub fn insert_string(&mut self, key: String, value: &str) {
        let variant = glib::Variant::from_str(value).unwrap();
        self.insert_variant(key, variant)
    }

    pub fn insert_bytes(&mut self, key: String, bytes: &[u8]) {
        let bytes = glib::Bytes::from(bytes);
        let variant = glib::Variant::from_bytes_with_type(&bytes, glib::VariantTy::BYTE_STRING);
        self.insert_variant(key, variant);
    }

    pub fn insert_table(&mut self, key: String, value: GvdbHashTableBuilder) {
        let item = GvdbBuilderValue::TableBuilder(value);
        self.insert(key, item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, (String, GvdbBuilderValue)> {
        self.items.iter_mut()
    }

    /*pub fn build(self) -> SimpleHashTable<GvdbItem> {
        let mut hash_table = SimpleHashTable::with_n_buckets(self.items.len());

        for (key, value) in self.items.into_iter() {
            let value = match value {
                GvdbBuilderValue::Value(val) => GvdbItemValue::Value(val),
                GvdbBuilderValue::TableBuilder(table_builder) => {
                    let table = table_builder.build();
                    GvdbItemValue::Table(table)
                }
            };

            //let item = GvdbItem::with_item_value(key.clone(), value);
            //let hash_value = hash_table.insert(&key, item);
        }

        hash_table
    }*/
}

pub enum GvdbItemValue {
    Value(glib::Variant),
    Table(SimpleHashTable<GvdbItemValue>),
}

/*
pub struct GvdbItem {
    key: String,
    hash_value: u32,
    next: OptLink<GvdbItem>,

    value: GvdbItemValue,
}

impl GvdbItem {
    fn with_item_value(key: String, value: GvdbItemValue) -> Self {
        let hash_value = djb_hash(&key);

        Self {
            key,
            hash_value,
            next: None,
            value,
        }
    }

    pub fn with_value(key: String, value: glib::Variant) -> Self {
        Self::with_item_value(key, GvdbItemValue::Value(value))
    }

    pub fn with_table(key: String, hash_value: u32, table: SimpleHashTable<GvdbItem>) -> Self {
        Self::with_item_value(key, GvdbItemValue::Table(table))
    }

    pub fn into_next(self) -> Option<Self> {
        self.next
    }
}*/

struct GvdbChunk {
    alignment: usize,
    pointer: GvdbPointer,
    size: usize,
    data: Vec<u8>,
}

impl GvdbChunk {
    pub fn allocate(file_builder: &mut GvdbFileBuilder, size: usize, alignment: usize) -> Self {
        file_builder.offset = align_offset(file_builder.offset, alignment);
        let offset_start = file_builder.offset;
        file_builder.offset += size;

        let pointer = GvdbPointer::new(offset_start, file_builder.offset);
        let data = Vec::with_capacity(size);

        Self {
            pointer,
            alignment,
            size,
            data,
        }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn alignment(&self) -> usize {
        self.alignment
    }

    pub fn pointer(&self) -> &GvdbPointer {
        &self.pointer
    }
}

struct GvdbFileBuilder {
    offset: usize,
    chunks: VecDeque<GvdbChunk>,
    byteswap: bool,
    table_builder: Option<GvdbHashTableBuilder>,
}

impl GvdbFileBuilder {
    pub fn new(byteswap: bool) -> Self {
        let table_builder = Some(GvdbHashTableBuilder::new());

        Self {
            offset: size_of::<GvdbHeader>(),
            chunks: Default::default(),
            byteswap,
            table_builder,
        }
    }

    pub fn table_builder(&mut self) -> &mut GvdbHashTableBuilder {
        self.table_builder.as_mut().unwrap()
    }

    /*pub fn build(mut self) -> Vec<u8> {
        self.offset = size_of::<GvdbHeader>();
        let table = self.table_builder.take().unwrap().build();

        let root = GvdbRoot::with_empty_header(self.byteswap);
        todo!()
    }*/

    /*fn add_variant(&mut self, variant: &glib::Variant) -> usize {
        let chunk = GvdbChunk::with_variant(self.offset, variant, self.byteswap);
        self.offset += chunk.padded_len();
        self.chunks.push_back(chunk);
        self.chunks.len() - 1
    }*/

    /*pub fn add_string(mut self, string: &str) -> usize {
        let data = string.as_bytes();
        let chunk = GvdbChunk::new(self.offset, data.to_vec(), 1);
        self.offset += chunk.padded_len();
        self.chunks.push_back(chunk);
        self.chunks.len() - 1
    }*/

    /*fn allocate_for_hash_table(&mut self, table: &SimpleHashTable<GvdbItem>) -> GvdbChunk {
        let header = GvdbHashHeader::new(0, table.buckets.len() as u32);
        todo!()
    }*/

    fn add_hash_table(&mut self, table: SimpleHashTable<Vec<u8>>) -> usize {
        let header = GvdbHashHeader::new(0, table.n_buckets() as u32);
        let items_len = table.n_items() * size_of::<GvdbHashItem>();
        let size = size_of::<GvdbHashHeader>()
            + header.bloom_words_len()
            + header.buckets_len()
            + items_len;

        let mut data: Vec<u8> = vec![0; size];
        data.extend_from_slice(transmute_one_to_bytes(&header));

        let mut bloom_words = vec![0; 0];
        data.append(&mut bloom_words);

        let mut buckets = Vec::with_capacity(table.n_buckets() as usize);
        let mut index = 0;
        for (n_bucket, bucket) in table.into_buckets().into_iter().enumerate() {
            buckets.push(index);

            let mut item = bucket;
            while let Some(current_item) = item {
                let key = current_item.key();
                // store_key
                //let hash_item = GvdbHashItem::new(current_item.hash(), 0, )

                item = current_item.into_next();
            }
        }

        unimplemented!()
    }

    pub fn serialize(&self, _root_index: usize) -> GvdbResult<Vec<u8>> {
        todo!()
        //let header = GvdbHeader::new(self.byteswap, 0, root);
    }

    /*pub fn with_variant(offset: usize, variant: &glib::Variant, byteswap: bool) -> Self {
        let value = if byteswap {
            glib::Variant::from_variant(&variant.byteswap())
        } else {
            glib::Variant::from_variant(variant)
        };

        let normal = glib::Variant::normal_form(&value);
        let mut data = Vec::new();

        normal
            .store(&mut data)
            .expect("glib::Variant::store failed");
        Self::new(offset, data, 8)
    }

    pub fn with_hash_table(_offset: usize, hash_table: SimpleHashTable<String>) -> Self {
        // The original C implementation doesn't fill the bloom filter so we don't either
        let _bloom_hdr: u32 = 0;
        let n_bloom_words: u32 = 0;
        let n_buckets = hash_table.n_buckets();
        let _n_items = hash_table.n_items;

        let _size = size_of::<u32>() // bloom_hdr
            + size_of::<u32>() // table_hdr
            + n_bloom_words as usize * size_of::<u32>()
            + n_buckets as usize * size_of::<u32>();

        todo!()
    }*/
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_hash_table() {
        let mut table: SimpleHashTable<String> = SimpleHashTable::with_n_buckets(10);
        table.insert("test", "test".to_string());
        assert_eq!(table.n_items, 1);
        assert_eq!(table.get("test").unwrap(), "test");
    }

    #[test]
    fn simple_hash_table_2() {
        let mut table: SimpleHashTable<u32> = SimpleHashTable::with_n_buckets(10);
        for index in 0..20 {
            table.insert(&format!("{}", index), index);
        }

        assert_eq!(table.n_items, 20);

        for index in 0..20 {
            let item = table.get(&format!("{}", index)).unwrap();
            assert_eq!(index, *item);
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
    pub fn gvdb_table_builder() {}
}
