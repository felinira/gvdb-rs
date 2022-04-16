use crate::read::hash_item::GvdbHashItemType;
use crate::write::builder::GvdbHashTableBuilder;
use glib::Variant;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

#[derive(Debug)]
pub enum GvdbBuilderItemValue {
    Value(glib::Variant),
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
            GvdbBuilderItemValue::TableBuilder(_) => GvdbHashItemType::HashTable,
            GvdbBuilderItemValue::Container(_) => GvdbHashItemType::Container,
        }
    }

    pub fn variant(&self) -> Option<&glib::Variant> {
        match self {
            GvdbBuilderItemValue::Value(variant) => Some(variant),
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

impl From<glib::Variant> for GvdbBuilderItemValue {
    fn from(var: Variant) -> Self {
        GvdbBuilderItemValue::Value(var)
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
