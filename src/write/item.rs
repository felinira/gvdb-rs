use crate::read::GvdbHashItemType;
use crate::write::file::GvdbHashTableBuilder;
use serde::Serialize;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;
use zvariant::{DynamicType, Value};

pub trait ZVariantConvertible {}
impl<T: ?Sized> ZVariantConvertible for T where T: Serialize + DynamicType {}

#[derive(Debug)]
pub enum GvdbBuilderItemValue {
    // A zvariant::Value
    Value(zvariant::Value<'static>),

    // A glib::Variant
    #[cfg(feature = "glib")]
    GVariant(glib::Variant),

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
            #[cfg(feature = "glib")]
            GvdbBuilderItemValue::GVariant(_) => GvdbHashItemType::Value,
            GvdbBuilderItemValue::TableBuilder(_) => GvdbHashItemType::HashTable,
            GvdbBuilderItemValue::Container(_) => GvdbHashItemType::Container,
        }
    }

    pub fn value(&self) -> Option<&Value> {
        match self {
            GvdbBuilderItemValue::Value(value) => Some(&value),
            _ => None,
        }
    }

    #[cfg(feature = "glib")]
    pub fn gvariant(&self) -> Option<&glib::Variant> {
        match self {
            GvdbBuilderItemValue::GVariant(variant) => Some(&variant),
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

impl From<Value<'static>> for GvdbBuilderItemValue {
    fn from(var: Value<'static>) -> Self {
        GvdbBuilderItemValue::Value(var)
    }
}

#[cfg(feature = "glib")]
impl From<glib::Variant> for GvdbBuilderItemValue {
    fn from(var: glib::Variant) -> Self {
        GvdbBuilderItemValue::GVariant(var)
    }
}

impl From<GvdbHashTableBuilder> for GvdbBuilderItemValue {
    fn from(tb: GvdbHashTableBuilder) -> Self {
        GvdbBuilderItemValue::TableBuilder(tb)
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

    pub fn next(&self) -> &RefCell<Option<Rc<GvdbBuilderItem>>> {
        &self.next
    }

    pub fn value(&self) -> &RefCell<GvdbBuilderItemValue> {
        &self.value
    }

    pub fn value_ref(&self) -> Ref<GvdbBuilderItemValue> {
        self.value.borrow()
    }

    pub fn value_mut(&self) -> RefMut<GvdbBuilderItemValue> {
        self.value.borrow_mut()
    }

    pub fn parent(&self) -> &RefCell<Option<Rc<GvdbBuilderItem>>> {
        &self.parent
    }

    pub fn parent_ref(&self) -> Ref<Option<Rc<GvdbBuilderItem>>> {
        self.parent.borrow()
    }

    pub fn parent_mut(&self) -> RefMut<Option<Rc<GvdbBuilderItem>>> {
        self.parent.borrow_mut()
    }

    pub fn assigned_index(&self) -> u32 {
        self.assigned_index.get()
    }

    pub fn set_assigned_index(&self, index: u32) {
        self.assigned_index.set(index);
    }
}
