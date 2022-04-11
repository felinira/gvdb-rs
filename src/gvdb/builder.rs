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

pub enum GvdbItemValue {
    Value(glib::Variant),
    Table(Link<GvdbBuilderHashTable>),
    Child(Link<GvdbItem>),
}

pub struct GvdbItem {
    pub key: String,
    hash_value: u32,
    assigned_index: u32,
    parent: OptLink<GvdbItem>,
    sibling: OptLink<GvdbItem>,
    pub next: OptLink<GvdbItem>,

    value: GvdbItemValue,
}

impl GvdbItem {
    fn with_item_value(key: String, value: GvdbItemValue) -> Self {
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
        Self::with_item_value(key, GvdbItemValue::Value(value))
    }

    pub fn with_table(key: String, table: Link<GvdbBuilderHashTable>) -> Self {
        Self::with_item_value(key, GvdbItemValue::Table(table))
    }

    pub fn with_child(key: String, child: Link<GvdbItem>) -> Self {
        Self::with_item_value(key, GvdbItemValue::Child(child))
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

    pub fn with_hash_table(_offset: usize, hash_table: SimpleHashTable) -> Self {
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

    fn add_hash(&mut self, table: Link<GvdbBuilderHashTable>) -> usize {
        let mut simple_hash_table = SimpleHashTable::with_capacity(table.borrow().len());

        let mut index = 0;
        for (key, value) in table.borrow_mut().iter_mut() {
            value.borrow_mut().assigned_index = index;
            simple_hash_table.put(key, Some(value.clone()));
            index += 1;
        }

        unimplemented!()
    }

    pub fn serialize(&self, _root_index: usize) -> GvdbResult<Vec<u8>> {
        todo!()
        //let header = GvdbHeader::new(self.byteswap, 0, root);
    }
}

struct SimpleHashTable {
    buckets: Vec<OptLink<GvdbItem>>,
    n_items: usize,
}

impl SimpleHashTable {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buckets: vec![None; capacity],
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

    pub fn from_hash_map(_table: HashMap<String, Vec<u8>>) -> Self {
        /*let this = Self::with_capacity(table.capacity());
        for (key, value) in table {}*/

        todo!();
    }

    pub fn put(&mut self, key: &str, value: OptLink<GvdbItem>) {
        let bucket = self.hash_bucket(key);

        if let Some(item) = value {
            // Add the item
            let replaced_item = std::mem::replace(&mut self.buckets[bucket], Some(item.clone()));
            if let Some(replaced_item) = replaced_item {
                if self.get_from_bucket(key, bucket).is_some() {
                    // Replace
                    item.borrow_mut().next = replaced_item.borrow().next.clone();
                } else {
                    // Insert
                    item.borrow_mut().next = Some(replaced_item);
                    self.n_items += 1;
                }
            }
        } else {
            // Remove the item if it already exists
            if let Some((previous, item)) = self.get_from_bucket(key, bucket) {
                self.n_items -= 1;

                if let Some(previous) = previous {
                    previous.borrow_mut().next = item.borrow().next.clone();
                } else {
                    self.buckets[bucket] = item.borrow().next.clone();
                }
            }
        }
    }

    fn get_from_bucket(
        &self,
        key: &str,
        bucket: usize,
    ) -> Option<(OptLink<GvdbItem>, Link<GvdbItem>)> {
        let mut item = self.buckets.get(bucket)?.clone();
        let mut previous_item = None;

        while let Some(current_item) = item {
            if current_item.borrow().key == key {
                return Some((previous_item, current_item.clone()));
            } else if let Some(next) = current_item.borrow().next.clone() {
                previous_item = Some(current_item.clone());
                item = Some(next);
            } else {
                item = None;
            }
        }

        None
    }

    pub fn get(&self, key: &str) -> OptLink<GvdbItem> {
        let bucket = self.hash_bucket(key);
        self.get_from_bucket(key, bucket).map(|t| t.1)
    }

    pub fn item_to_index(item: OptLink<GvdbItem>) -> u32 {
        match item {
            None => u32::MAX,
            Some(item) => item.borrow().assigned_index,
        }
    }
}

pub struct GvdbBuilderHashTable {
    hash_map: HashMap<String, Link<GvdbItem>>,
}

impl GvdbBuilderHashTable {
    pub fn new() -> Link<Self> {
        Link::new(Self {
            hash_map: Default::default(),
        })
    }

    pub fn with_parent(
        &self,
        parent: Link<GvdbBuilderHashTable>,
        name_in_parent: &str,
    ) -> Link<Self> {
        let this = Self::new();
        let value = Link::new(GvdbItem::with_table(
            name_in_parent.to_string(),
            this.clone(),
        ));
        parent
            .borrow_mut()
            .insert(name_in_parent.to_string(), value);

        this
    }

    fn insert(&mut self, key: String, item: Link<GvdbItem>) {
        self.hash_map.insert(key, item);
    }

    pub fn insert_variant(&mut self, key: String, variant: glib::Variant) {
        let item = Link::new(GvdbItem::with_value(key.clone(), variant));
        self.insert(key, item);
    }

    pub fn insert_string(&mut self, key: String, value: &str) {
        let variant = glib::Variant::from_str(value).unwrap();
        self.insert_variant(key, variant)
    }

    pub fn len(&self) -> usize {
        self.hash_map.len()
    }

    pub fn hash_map(&self) -> &HashMap<String, Link<GvdbItem>> {
        &self.hash_map
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, String, Link<GvdbItem>> {
        self.hash_map.iter_mut()
    }
}
