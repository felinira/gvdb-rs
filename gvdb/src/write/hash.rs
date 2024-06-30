use crate::util::djb_hash;
use crate::write::item::{HashItemBuilder, HashValue};
use std::rc::Rc;

/// A hash table with a fixed number of buckets.
///
/// This is used as an intermediate representation before serializing
/// hashtable data in a HVDB file.
#[derive(Debug)]
pub struct SimpleHashTable<'a> {
    buckets: Vec<Option<Rc<HashItemBuilder<'a>>>>,
    n_items: usize,
}

impl<'a> SimpleHashTable<'a> {
    /// Create a hash table with a number of buckets.
    pub fn with_n_buckets(n_buckets: usize) -> Self {
        let mut buckets = Vec::with_capacity(n_buckets);
        buckets.resize_with(n_buckets, || None);

        Self {
            buckets,
            n_items: 0,
        }
    }

    /// The number of buckets of the hash table. This number is fixed and does not change.
    pub fn n_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// How many items are contained in the hash table.
    pub fn n_items(&self) -> usize {
        self.n_items
    }

    /// Retrieve the hash bucket for the provided [`u32`] hash value
    fn hash_bucket(&self, hash_value: u32) -> usize {
        (hash_value % self.buckets.len() as u32) as usize
    }

    /// Insert a new item into the hash table.
    ///
    /// Returns the created hash item.
    pub fn insert(&mut self, key: &str, item: HashValue<'a>) -> Rc<HashItemBuilder<'a>> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);

        let item = Rc::new(HashItemBuilder::new(key, hash_value, item));
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
    pub fn remove(&mut self, key: &str) -> bool {
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

            true
        } else {
            false
        }
    }

    /// Retrieve an item with the specified key from the specified bucket.
    fn get_from_bucket(
        &self,
        key: &str,
        bucket: usize,
    ) -> Option<(Option<Rc<HashItemBuilder<'a>>>, Rc<HashItemBuilder<'a>>)> {
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

    /// Returns an item corresponding to the key.
    pub fn get(&self, key: &str) -> Option<Rc<HashItemBuilder<'a>>> {
        let hash_value = djb_hash(key);
        let bucket = self.hash_bucket(hash_value);
        self.get_from_bucket(key, bucket).map(|r| r.1)
    }

    /// Iterator over the hash table items.
    pub fn iter(&self) -> SimpleHashTableIter<'_, 'a> {
        SimpleHashTableIter {
            hash_table: self,
            bucket: 0,
            last_item: None,
        }
    }

    /// Iterator over the items in the specified bucket.
    pub fn iter_bucket(&self, bucket: usize) -> SimpleHashTableBucketIter<'_, 'a> {
        SimpleHashTableBucketIter {
            hash_table: self,
            bucket,
            last_item: None,
        }
    }
}

/// Iterator over the items in a specific bucket of a [`SimpleHashTable`].
pub struct SimpleHashTableBucketIter<'it, 'h> {
    hash_table: &'it SimpleHashTable<'h>,
    bucket: usize,
    last_item: Option<Rc<HashItemBuilder<'h>>>,
}

impl<'it, 'h> Iterator for SimpleHashTableBucketIter<'it, 'h> {
    type Item = Rc<HashItemBuilder<'h>>;

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
        } else if let Some(Some(item)) = self.hash_table.buckets.get(self.bucket).cloned() {
            // We found something: Bucket exists and is not empty
            self.last_item = Some(item.clone());
            Some(item.clone())
        } else {
            None
        }
    }
}

/// Iterator over the items of a [`SimpleHashTable`].
pub struct SimpleHashTableIter<'it, 'h> {
    hash_table: &'it SimpleHashTable<'h>,
    bucket: usize,
    last_item: Option<Rc<HashItemBuilder<'h>>>,
}

impl<'it, 'h> Iterator for SimpleHashTableIter<'it, 'h> {
    type Item = (usize, Rc<HashItemBuilder<'h>>);

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

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use matches::assert_matches;

    use crate::write::hash::SimpleHashTable;
    use crate::write::item::HashValue;

    #[test]
    fn derives() {
        let table = SimpleHashTable::with_n_buckets(1);
        assert!(format!("{:?}", table).contains("SimpleHashTable"));
    }

    #[test]
    fn simple_hash_table() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        let item = HashValue::Value(zvariant::Value::new("test_overwrite"));
        table.insert("test", item);
        assert_eq!(table.n_items(), 1);
        let item2 = HashValue::Value(zvariant::Value::new("test"));
        table.insert("test", item2);
        assert_eq!(table.n_items(), 1);
        assert_eq!(
            table.get("test").unwrap().value_ref().value().unwrap(),
            &"test".into()
        );
    }

    #[test]
    fn simple_hash_table_2() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        for index in 0..20 {
            table.insert(&format!("{}", index), zvariant::Value::new(index).into());
        }

        assert_eq!(table.n_items(), 20);

        for index in 0..20 {
            assert_eq!(
                zvariant::Value::new(index),
                *table
                    .get(&format!("{}", index))
                    .unwrap()
                    .value_ref()
                    .value()
                    .unwrap()
            );
        }

        for index in 0..10 {
            let index = index * 2;
            assert!(table.remove(&format!("{}", index)));
        }

        for index in 0..20 {
            let item = table.get(&format!("{}", index));
            assert_eq!(index % 2 == 1, item.is_some());
        }

        assert!(!table.remove("50"));
    }

    #[test]
    fn simple_hash_table_iter() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        for index in 0..20 {
            table.insert(&format!("{}", index), zvariant::Value::new(index).into());
        }

        let mut iter = table.iter();
        for _ in 0..20 {
            let value: i32 = iter
                .next()
                .unwrap()
                .1
                .value()
                .borrow()
                .value()
                .unwrap()
                .try_into()
                .unwrap();
            assert_matches!(value, 0..=19);
        }
    }

    #[test]
    fn simple_hash_table_bucket_iter() {
        let mut table: SimpleHashTable = SimpleHashTable::with_n_buckets(10);
        for index in 0..20 {
            table.insert(&format!("{}", index), zvariant::Value::new(index).into());
        }

        let mut values: HashSet<i32> = (0..20).collect();
        for bucket in 0..table.n_buckets() {
            let iter = table.iter_bucket(bucket);
            for next in iter {
                let num: i32 = next.value().borrow().value().unwrap().try_into().unwrap();
                assert!(values.remove(&num));
            }
        }
    }
}
