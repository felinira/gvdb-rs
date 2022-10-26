use crate::read::GvdbHashItemType;
use crate::write::file::GvdbHashTableBuilder;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

#[derive(Debug)]
pub enum GvdbBuilderItemValue<'a> {
    // A zvariant::Value
    Value(zvariant::Value<'a>),

    // A glib::Variant
    #[cfg(feature = "glib")]
    GVariant(glib::Variant),

    TableBuilder(GvdbHashTableBuilder<'a>),

    // A child container with no additional value
    Container(Vec<String>),
}

impl<'a> Default for GvdbBuilderItemValue<'a> {
    fn default() -> Self {
        Self::Container(Vec::new())
    }
}

#[allow(dead_code)]
impl<'a> GvdbBuilderItemValue<'a> {
    pub fn typ(&self) -> GvdbHashItemType {
        match self {
            GvdbBuilderItemValue::Value(_) => GvdbHashItemType::Value,
            #[cfg(feature = "glib")]
            GvdbBuilderItemValue::GVariant(_) => GvdbHashItemType::Value,
            GvdbBuilderItemValue::TableBuilder(_) => GvdbHashItemType::HashTable,
            GvdbBuilderItemValue::Container(_) => GvdbHashItemType::Container,
        }
    }

    pub fn value(&self) -> Option<&zvariant::Value> {
        match self {
            GvdbBuilderItemValue::Value(value) => Some(value),
            _ => None,
        }
    }

    #[cfg(feature = "glib")]
    pub fn gvariant(&self) -> Option<&glib::Variant> {
        match self {
            GvdbBuilderItemValue::GVariant(variant) => Some(variant),
            _ => None,
        }
    }

    #[allow(dead_code)]
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

impl<'a> From<zvariant::Value<'a>> for GvdbBuilderItemValue<'a> {
    fn from(var: zvariant::Value<'a>) -> Self {
        GvdbBuilderItemValue::Value(var)
    }
}

#[cfg(feature = "glib")]
impl<'a> From<glib::Variant> for GvdbBuilderItemValue<'a> {
    fn from(var: glib::Variant) -> Self {
        GvdbBuilderItemValue::GVariant(var)
    }
}

impl<'a> From<GvdbHashTableBuilder<'a>> for GvdbBuilderItemValue<'a> {
    fn from(tb: GvdbHashTableBuilder<'a>) -> Self {
        GvdbBuilderItemValue::TableBuilder(tb)
    }
}

#[derive(Debug)]
pub struct GvdbBuilderItem<'a> {
    // The key string of the item
    key: String,

    // The djb hash
    hash: u32,

    // An arbitrary data container
    value: RefCell<GvdbBuilderItemValue<'a>>,

    // The assigned index for the gvdb file
    assigned_index: Cell<u32>,

    // The parent item of this builder item
    parent: RefCell<Option<Rc<GvdbBuilderItem<'a>>>>,

    // The next item in the hash bucket
    next: RefCell<Option<Rc<GvdbBuilderItem<'a>>>>,
}

impl<'a> GvdbBuilderItem<'a> {
    pub fn new(key: &str, hash: u32, value: GvdbBuilderItemValue<'a>) -> Self {
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

    pub fn next(&self) -> &RefCell<Option<Rc<GvdbBuilderItem<'a>>>> {
        &self.next
    }

    pub fn value(&self) -> &RefCell<GvdbBuilderItemValue<'a>> {
        &self.value
    }

    pub fn value_ref(&self) -> Ref<GvdbBuilderItemValue<'a>> {
        self.value.borrow()
    }

    pub fn parent(&self) -> &RefCell<Option<Rc<GvdbBuilderItem<'a>>>> {
        &self.parent
    }

    pub fn parent_ref(&self) -> Ref<Option<Rc<GvdbBuilderItem<'a>>>> {
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
    use crate::read::GvdbHashItemType;
    use crate::write::item::{GvdbBuilderItem, GvdbBuilderItemValue};
    use crate::write::GvdbHashTableBuilder;
    use matches::assert_matches;

    #[test]
    fn derives() {
        let value1: zvariant::Value = "test".into();
        let item1 = GvdbBuilderItemValue::Value(value1.clone());
        println!("{:?}", item1);
    }

    #[test]
    fn item_value() {
        let value1: zvariant::Value = "test".into();
        let item1 = GvdbBuilderItemValue::Value(value1.clone());
        assert_eq!(item1.typ(), GvdbHashItemType::Value);
        assert_eq!(item1.value().unwrap(), &value1);

        let value2 = GvdbHashTableBuilder::new();
        let item2 = GvdbBuilderItemValue::from(value2);
        assert_eq!(item2.typ(), GvdbHashItemType::HashTable);
        assert!(item2.table_builder().is_some());

        let value3 = vec!["test".to_string(), "test2".to_string()];
        let item3 = GvdbBuilderItemValue::Container(value3.clone());
        assert_eq!(item3.typ(), GvdbHashItemType::Container);
        assert_eq!(item3.container().unwrap(), &value3);
    }

    #[test]
    fn builder_item() {
        let value1: zvariant::Value = "test".into();
        let item1 = GvdbBuilderItemValue::Value(value1.clone());
        let item = GvdbBuilderItem::new("test", 0, item1);
        println!("{:?}", item);

        assert_eq!(item.key(), "test");
        assert_matches!(&*item.value().borrow(), GvdbBuilderItemValue::Value(_));
    }
}

#[cfg(all(feature = "glib", test))]
mod test_glib {
    use crate::read::GvdbHashItemType;
    use crate::write::item::GvdbBuilderItemValue;
    use glib::ToVariant;

    #[test]
    fn item_value() {
        let value1 = "test".to_variant();
        let item1 = GvdbBuilderItemValue::from(value1.clone());
        assert_eq!(item1.typ(), GvdbHashItemType::Value);
        assert_eq!(item1.gvariant().unwrap(), &value1);
    }
}
