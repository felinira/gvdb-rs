use crate::read::HashItemType;
#[cfg(feature = "zvariant")]
use crate::variant::EncodeVariant;
use crate::write::file::HashTableBuilder;
use std::cell::{Cell, Ref, RefCell};
use std::convert::Infallible;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;

/// Holds the value of a GVDB hash table
#[derive(Debug)]
#[non_exhaustive]
pub enum HashValue<'a> {
    /// A serialized gvariant value
    #[cfg(feature = "zvariant")]
    Value(Box<dyn EncodeVariant<'a> + 'a>),

    /// A glib::Variant
    #[cfg(feature = "glib")]
    GVariant(glib::Variant),

    TableBuilder(HashTableBuilder<'a>),

    /// A child container with no additional value
    Container(Vec<String>),

    #[doc(hidden)]
    _Placeholder((Infallible, PhantomData<&'a str>)),
}

impl Default for HashValue<'_> {
    fn default() -> Self {
        Self::Container(Vec::new())
    }
}

#[allow(dead_code)]
impl<'a> HashValue<'a> {
    #[cfg(feature = "zvariant")]
    pub fn from_value<T: EncodeVariant<'a> + 'a>(value: T) -> Self {
        Self::Value(Box::new(value))
    }

    pub fn typ(&self) -> HashItemType {
        match self {
            #[cfg(feature = "zvariant")]
            HashValue::Value(_) => HashItemType::Value,
            #[cfg(feature = "glib")]
            HashValue::GVariant(_) => HashItemType::Value,
            HashValue::TableBuilder(_) => HashItemType::HashTable,
            HashValue::Container(_) => HashItemType::Container,
            HashValue::_Placeholder((_, _)) => unreachable!(),
        }
    }

    #[cfg(test)]
    pub(crate) fn encode_value(&self, endian: crate::Endian) -> crate::write::Result<Box<[u8]>> {
        match self {
            #[cfg(feature = "zvariant")]
            HashValue::Value(value) => Ok(value.encode(endian)?),
            #[cfg(feature = "glib")]
            HashValue::GVariant(variant) => {
                let variant = if endian.is_byteswap() {
                    &variant.byteswap()
                } else {
                    variant
                };

                Ok(variant.data().into())
            }
            _ => Err(crate::write::Error::Consistency(
                "Expected type 'Value'".to_string(),
            )),
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
    pub fn table_builder(&self) -> Option<&HashTableBuilder<'a>> {
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

#[cfg(feature = "glib")]
impl From<glib::Variant> for HashValue<'_> {
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
    /// The key string of the item
    key: String,

    /// The djb hash
    hash: u32,

    /// An arbitrary data container
    value: RefCell<HashValue<'a>>,

    /// The assigned index for the gvdb file
    assigned_index: Cell<u32>,

    /// The parent item of this builder item
    parent: RefCell<Option<Rc<HashItemBuilder<'a>>>>,

    /// The next item in the hash bucket
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

    pub fn value_ref(&self) -> Ref<'_, HashValue<'a>> {
        self.value.borrow()
    }

    pub fn parent(&self) -> &RefCell<Option<Rc<HashItemBuilder<'a>>>> {
        &self.parent
    }

    pub fn parent_ref(&self) -> Ref<'_, Option<Rc<HashItemBuilder<'a>>>> {
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
#[allow(unused_imports)]
mod test {
    use crate::Endian;
    use crate::read::HashItemType;
    #[cfg(feature = "zvariant")]
    use crate::variant::EncodeVariant;
    use crate::write::HashTableBuilder;
    use crate::write::item::{HashItemBuilder, HashValue};
    use matches::assert_matches;

    #[test]
    #[cfg(feature = "zvariant")]
    fn derives() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::from_value(value1);
        println!("{item1:?}");
    }

    #[test]
    #[cfg(feature = "zvariant")]
    fn item_value() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::Value(Box::new(
            value1
                .try_clone()
                .expect("Value to not contain a file descriptor"),
        ));
        assert_eq!(item1.typ(), HashItemType::Value);
        assert_eq!(
            item1.encode_value(Endian::Little).unwrap(),
            value1.encode(Endian::Little).unwrap()
        );

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
    #[cfg(feature = "zvariant")]
    fn builder_item() {
        let value1: zvariant::Value = "test".into();
        let item1 = HashValue::from_value(value1);
        let item = HashItemBuilder::new("test", 0, item1);
        println!("{item:?}");

        assert_eq!(item.key(), "test");
        assert_matches!(&*item.value().borrow(), HashValue::Value(_));
    }
}

#[cfg(all(feature = "glib", test))]
#[allow(unused_imports)]
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
        assert_eq!(
            glib::Variant::from_data::<&str, _>(
                &item1.encode_value(crate::Endian::NATIVE).unwrap()
            ),
            value1
        );
    }
}
