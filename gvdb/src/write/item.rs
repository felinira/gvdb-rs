use crate::read::HashItemType;
use crate::write::file::HashTableBuilder;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// Holds the value of a GVDB hash table
#[derive(Debug)]
pub enum HashValue<'a> {
    // A zvariant::Value
    Value(zvariant::Value<'a>),

    // A glib::Variant
    #[cfg(feature = "glib")]
    GVariant(glib::Variant),

    TableBuilder(HashTableBuilder<'a>),

    // A child container with no additional value
    Container(Vec<String>),
}

impl<'a> Default for HashValue<'a> {
    fn default() -> Self {
        Self::Container(Vec::new())
    }
}

#[allow(dead_code)]
impl<'a> HashValue<'a> {
    pub fn typ(&self) -> HashItemType {
        match self {
            HashValue::Value(_) => HashItemType::Value,
            #[cfg(feature = "glib")]
            HashValue::GVariant(_) => HashItemType::Value,
            HashValue::TableBuilder(_) => HashItemType::HashTable,
            HashValue::Container(_) => HashItemType::Container,
        }
    }

    pub fn value(&self) -> Option<&zvariant::Value> {
        match self {
            HashValue::Value(value) => Some(value),
            _ => None,
        }
    }

    #[cfg(feature = "glib")]
    pub fn gvariant(&self) -> Option<&glib::Variant> {
        match self {
            HashValue::GVariant(variant) => Some(variant),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn table_builder(&self) -> Option<&HashTableBuilder> {
        match self {
            HashValue::TableBuilder(tb) => Some(tb),
            _ => None,
        }
    }

    pub fn container(&self) -> Option<&Vec<String>> {
        match self {
            HashValue::Container(children) => Some(children),
            _ => None,
        }
    }
}

impl<'a> From<zvariant::Value<'a>> for HashValue<'a> {
    fn from(var: zvariant::Value<'a>) -> Self {
        HashValue::Value(var)
    }
}

#[cfg(feature = "glib")]
impl<'a> From<glib::Variant> for HashValue<'a> {
    fn from(var: glib::Variant) -> Self {
        HashValue::GVariant(var)
    }
}

impl<'a> From<HashTableBuilder<'a>> for HashValue<'a> {
    fn from(tb: HashTableBuilder<'a>) -> Self {
        HashValue::TableBuilder(tb)
    }
}

#[derive(Debug)]
pub struct HashItemBuilder<'a> {
    // The key string of the item
    key: String,

    // The djb hash
    hash: u32,

    // An arbitrary data container
    value: RefCell<HashValue<'a>>,

    // The assigned index for the gvdb file
    assigned_index: Cell<u32>,

    // The parent item of this builder item
    parent: RefCell<Option<Rc<HashItemBuilder<'a>>>>,

    // The next item in the hash bucket
    next: RefCell<Option<Rc<HashItemBuilder<'a>>>>,
}

impl<'a> HashItemBuilder<'a> {
    pub fn new(key: &str, hash: u32, value: HashValue<'a>) -> Self {
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

    pub fn next(&self) -> &RefCell<Option<Rc<HashItemBuilder<'a>>>> {
        &self.next
    }

    pub fn value(&self) -> &RefCell<HashValue<'a>> {
        &self.value
    }

    pub fn value_ref(&self) -> Ref<HashValue<'a>> {
        self.value.borrow()
    }

    pub fn parent(&self) -> &RefCell<Option<Rc<HashItemBuilder<'a>>>> {
        &self.parent
    }

    pub fn parent_ref(&self) -> Ref<Option<Rc<HashItemBuilder<'a>>>> {
        self.parent.borrow()
    }

    pub fn assigned_index(&self) -> u32 {
        self.assigned_index.get()
    }

    pub fn set_assigned_index(&self, index: u32) {
        self.assigned_index.set(index);
    }
}

#[cfg(test)]
mod test {
    use crate::read::HashItemType;
    use crate::write::item::{HashItemBuilder, HashValue};
    use crate::write::HashTableBuilder;
    use matches::assert_matches;

    #[test]
    fn derives() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::Value(value1);
        println!("{:?}", item1);
    }

    #[test]
    fn item_value() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::Value(
            value1
                .try_clone()
                .expect("Value to not contain a file descriptor"),
        );
        assert_eq!(item1.typ(), HashItemType::Value);
        assert_eq!(item1.value().unwrap(), &value1);

        #[cfg(feature = "glib")]
        assert_matches!(item1.gvariant(), None);

        let value2 = HashTableBuilder::new();
        let item2 = HashValue::from(value2);
        assert_eq!(item2.typ(), HashItemType::HashTable);
        assert!(item2.table_builder().is_some());
        assert_matches!(item2.container(), None);

        let value3 = vec!["test".to_string(), "test2".to_string()];
        let item3 = HashValue::Container(value3.clone());
        assert_eq!(item3.typ(), HashItemType::Container);
        assert_eq!(item3.container().unwrap(), &value3);
        assert_matches!(item3.table_builder(), None);
    }

    #[test]
    fn builder_item() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::Value(value1);
        let item = HashItemBuilder::new("test", 0, item1);
        println!("{:?}", item);

        assert_eq!(item.key(), "test");
        assert_matches!(&*item.value().borrow(), HashValue::Value(_));
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::HashItemType;
    use crate::write::item::HashValue;
    use glib::prelude::*;
    use matches::assert_matches;

    #[test]
    fn item_value() {
        let value1 = "test".to_variant();
        let item1 = HashValue::from(value1.clone());
        assert_eq!(item1.typ(), HashItemType::Value);
        assert_eq!(item1.gvariant().unwrap(), &value1);
        assert_matches!(item1.value(), None);
    }
}
