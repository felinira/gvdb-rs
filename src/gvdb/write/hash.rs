use crate::gvdb::util::djb_hash;
use crate::gvdb::write::item::{GvdbBuilderItem, GvdbBuilderItemValue};
use std::iter::Map;
use std::rc::Rc;

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
            if replaced_item.key() == key {
                // Replace
                self.buckets[bucket]
                    .as_ref()
                    .unwrap()
                    .next()
                    .replace(replaced_item.next().take());
            } else {
                // Insert
                self.buckets[bucket]
                    .as_ref()
                    .unwrap()
                    .next()
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
                previous.next().replace(item.next().take());
            } else {
                self.buckets[bucket] = item.next().take();
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
            if current_item.key() == key {
                return Some((previous, current_item.clone()));
            } else {
                previous = Some(current_item.clone());
                item = current_item.next().borrow().clone();
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
            if let Some(next_item) = &*last_item.next().borrow() {
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
