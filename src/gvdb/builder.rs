use crate::gvdb::error::GvdbResult;
use crate::gvdb::header::GvdbHeader;
use crate::gvdb::pointer::GvdbPointer;
use crate::gvdb::util::djb_hash;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
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

pub struct GvdbItem {
    pub key: String,
    hash_value: u32,
    assigned_index: u32,
    parent: OptLink<GvdbItem>,
    sibling: OptLink<GvdbItem>,
    pub next: OptLink<GvdbItem>,

    value: GvdbBuilderValue,
}

impl GvdbItem {
    fn with_item_value(key: String, value: GvdbBuilderValue) -> Self {
        let hash_value = djb_hash(&key);

        Self {
            key,
            hash_value,
            assigned_index: 0,
            parent: None,
            sibling: None,
            next: None,
            value,
        }
    }

    pub fn with_value(key: String, value: glib::Variant) -> Self {
        Self::with_item_value(key, GvdbBuilderValue::Value(value))
    }

    pub fn with_table(key: String, table: GvdbHashTableBuilder) -> Self {
        Self::with_item_value(key, GvdbBuilderValue::TableBuilder(table))
    }

    pub fn assigned_index(&self) -> u32 {
        u32::from_le(self.assigned_index)
    }

    pub fn set_assigned_index(&mut self, assigned_index: u32) {
        self.assigned_index = assigned_index.to_le();
    }
}

struct GvdbChunk {
    alignment: usize,
    pointer: GvdbPointer,
    data: Vec<u8>,
}

impl GvdbChunk {
    pub fn new(offset: usize, data: Vec<u8>, alignment: usize) -> Self {
        let pointer = GvdbPointer::new(offset, offset + data.len());

        Self {
            pointer,
            alignment,
            data,
        }
    }

    pub fn with_variant(offset: usize, variant: &glib::Variant, byteswap: bool) -> Self {
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
    }

    pub fn alignment(&self) -> usize {
        self.alignment
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn inner_len(&self) -> usize {
        self.data.len()
    }

    pub fn padded_len(&self) -> usize {
        let len = self.data.len();
        len + len % self.alignment
    }

    pub fn pointer(&self) -> &GvdbPointer {
        &self.pointer
    }
}

struct GvdbFileBuilder {
    offset: usize,
    chunks: VecDeque<GvdbChunk>,
    byteswap: bool,
}

impl GvdbFileBuilder {
    pub fn new(byteswap: bool) -> Self {
        Self {
            offset: size_of::<GvdbHeader>(),
            chunks: Default::default(),
            byteswap,
        }
    }
    /*
        pub fn add_variant(&mut self, variant: &glib::Variant) -> usize {
            let chunk = GvdbChunk::with_variant(self.offset, variant, self.byteswap);
            self.offset += chunk.padded_len();
            self.chunks.push_back(chunk);
            self.chunks.len() - 1
        }

        pub fn add_string(mut self, string: &str) -> usize {
            let data = string.as_bytes();
            let chunk = GvdbChunk::new(self.offset, data.to_vec(), 1);
            self.offset += chunk.padded_len();
            self.chunks.push_back(chunk);
            self.chunks.len() - 1
        }

        fn add_hash(&mut self, table: Link<GvdbHashTableBuilder>) -> usize {
            let mut simple_hash_table = SimpleHashTable::with_n_buckets(table.borrow().len());

            let mut index = 0;
            for (key, value) in table.borrow_mut().iter_mut() {
                value.borrow_mut().assigned_index = index;
                simple_hash_table.put(key, Some(value.clone()));
                index += 1;
            }

            unimplemented!()
        }
    */
    pub fn serialize(&self, _root_index: usize) -> GvdbResult<Vec<u8>> {
        todo!()
        //let header = GvdbHeader::new(self.byteswap, 0, root);
    }
}

#[derive(Clone)]
struct SimpleHashTableItem<T: Clone> {
    key: String,
    value: T,
    next: Option<Box<SimpleHashTableItem<T>>>,
}

impl<T: Clone> SimpleHashTableItem<T> {
    pub fn new(key: String, value: T) -> Self {
        Self {
            key,
            value,
            next: None,
        }
    }

    pub fn value_ref(&self) -> &T {
        &self.value
    }
}

struct SimpleHashTable<T: Clone> {
    buckets: Vec<Option<Box<SimpleHashTableItem<T>>>>,
    n_items: usize,
}

impl<T: Clone> SimpleHashTable<T> {
    pub fn with_n_buckets(n_buckets: usize) -> Self {
        Self {
            buckets: vec![None; n_buckets],
            n_items: 0,
        }
    }

    pub fn n_buckets(&self) -> usize {
        self.buckets.len()
    }

    pub fn n_items(&self) -> usize {
        self.n_items
    }

    fn hash_bucket(&self, key: &str) -> usize {
        let hash_value = djb_hash(key);
        (hash_value % self.buckets.len() as u32) as usize
    }

    pub fn from_hash_map(table: HashMap<String, T>) -> Self {
        let mut this = Self::with_n_buckets(table.capacity());
        for (key, value) in table {
            this.insert(&key, value);
        }

        this
    }

    /// Insert the item for the specified key
    pub fn insert(&mut self, key: &str, item: T) {
        let bucket = self.hash_bucket(key);

        let item = SimpleHashTableItem::new(key.to_string(), item);
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
        let bucket = self.hash_bucket(key);

        // Remove the item if it already exists
        if let Some((item, previous)) = self.get_from_bucket(key, bucket) {
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

    fn get_from_bucket(
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

    pub fn get(&mut self, key: &str) -> Option<&T> {
        let bucket = self.hash_bucket(key);
        self.get_from_bucket(key, bucket)
            .and_then(|(item, previous)| {
                if previous {
                    item.next.as_ref()
                } else {
                    Some(item)
                }
            })
            .map(|x| x.value_ref())
    }

    pub fn item_to_index(item: OptLink<GvdbItem>) -> u32 {
        match item {
            None => u32::MAX,
            Some(item) => item.borrow().assigned_index,
        }
    }
}

pub struct GvdbHashTableBuilder {
    hash_map: HashMap<String, GvdbBuilderValue>,
}

impl GvdbHashTableBuilder {
    pub fn new() -> Link<Self> {
        Link::new(Self {
            hash_map: Default::default(),
        })
    }

    fn insert(&mut self, key: String, item: GvdbBuilderValue) {
        self.hash_map.insert(key, item);
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
        self.hash_map.len()
    }

    pub fn hash_map(&self) -> &HashMap<String, GvdbBuilderValue> {
        &self.hash_map
    }

    pub fn iter_mut(
        &mut self,
    ) -> std::collections::hash_map::IterMut<'_, String, GvdbBuilderValue> {
        self.hash_map.iter_mut()
    }
}

#[cfg(test)]
mod test {
    use crate::gvdb::builder::SimpleHashTable;

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
}
