use crate::util::djb_hash;
use crate::write::item::{GvdbBuilderItem, GvdbBuilderItemValue};
use std::rc::Rc;

pub struct SimpleHashTable<'a> {
    buckets: Vec<Option<Rc<GvdbBuilderItem<'a>>>>,
    n_items: usize,
}

impl<'a> SimpleHashTable<'a> {
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

    pub fn insert(&mut self, key: &str, item: GvdbBuilderItemValue<'a>) -> Rc<GvdbBuilderItem<'a>> {
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

    #[allow(dead_code)]
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
    ) -> Option<(Option<Rc<GvdbBuilderItem<'a>>>, Rc<GvdbBuilderItem<'a>>)> {
        let mut item = self.buckets.get(bucket)?.clone();
        let mut previous = None;

        while let Some(current_item) = item {
            if current_item.key() == key {
                return Some((previous, current_item));
            } else {
                previous = Some(current_item.clone());
                item = current_item.next().borrow().clone();
            }
        }

        None
    }

    pub fn get(&self, key: &str) -> Option<Rc<GvdbBuilderItem<'a>>> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);
        self.get_from_bucket(key, bucket).map(|r| r.1)
    }

    #[allow(dead_code)]
    pub fn into_buckets(self) -> Vec<Option<Rc<GvdbBuilderItem<'a>>>> {
        self.buckets
    }

    pub fn iter(&self) -> SimpleHashTableIter<'_, 'a> {
        SimpleHashTableIter {
            hash_table: self,
            bucket: 0,
            last_item: None,
        }
    }

    pub fn iter_bucket(&self, bucket: usize) -> SimpleHashTableBucketIter<'_, 'a> {
        SimpleHashTableBucketIter {
            hash_table: self,
            bucket,
            last_item: None,
        }
    }
}

pub struct SimpleHashTableBucketIter<'it, 'h> {
    hash_table: &'it SimpleHashTable<'h>,
    bucket: usize,
    last_item: Option<Rc<GvdbBuilderItem<'h>>>,
}

impl<'it, 'h> Iterator for SimpleHashTableBucketIter<'it, 'h> {
    type Item = Rc<GvdbBuilderItem<'h>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(last_item) = self.last_item.clone() {
            // First check if there are more items in this bucket
            if let Some(next_item) = &*last_item.next().borrow() {
                // Next item in the same bucket
                self.last_item = Some(next_item.clone());
                Some(next_item.clone())
            } else {
                // Last item in the bucket, return
                None
            }
        } else {
            if let Some(bucket_item) = self.hash_table.buckets.get(self.bucket).cloned() {
                // This bucket might be empty
                if let Some(item) = bucket_item {
                    // We found something
                    self.last_item = Some(item.clone());
                    Some(item.clone())
                } else {
                    // Empty bucket, return None
                    None
                }
            } else {
                None
            }
        }
    }
}

pub struct SimpleHashTableIter<'it, 'h> {
    hash_table: &'it SimpleHashTable<'h>,
    bucket: usize,
    last_item: Option<Rc<GvdbBuilderItem<'h>>>,
}

impl<'it, 'h> Iterator for SimpleHashTableIter<'it, 'h> {
    type Item = (usize, Rc<GvdbBuilderItem<'h>>);

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
/*
impl<'a, 'b> IntoIterator for &'a SimpleHashTable<'b> {
    type Item = (usize, Rc<GvdbBuilderItem<'a>>);
    type IntoIter = SimpleHashTableIter<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
*/
