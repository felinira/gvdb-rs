use crate::gvdb::hash::GvdbHashTable;
use crate::gvdb::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;

pub enum GvdbValue<'a> {
    Variant(glib::Variant),
    HashTable(GvdbHashTable<'a>),
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GvdbHashItem {
    hash_value: u32,
    parent: u32,

    key_start: u32,
    key_size: u16,

    typ: u8,
    unused: u8,

    // no endianness here otherwise we would need context
    value: GvdbPointer,
}

unsafe impl TriviallyTransmutable for GvdbHashItem {}

impl GvdbHashItem {
    pub fn new(
        hash_value: u32,
        parent: u32,
        key_start: u32,
        key_size: u16,
        typ: u8,
        value: GvdbPointer,
    ) -> Self {
        Self {
            hash_value,
            parent,
            key_start,
            key_size,
            typ,
            unused: 0,
            value,
        }
    }

    pub fn hash_value(&self) -> u32 {
        u32::from_le(self.hash_value)
    }

    pub fn parent(&self) -> u32 {
        u32::from_le(self.parent)
    }

    pub fn key_start(&self) -> u32 {
        u32::from_le(self.key_start)
    }

    pub fn key_size(&self) -> u16 {
        u16::from_le(self.key_size)
    }

    pub fn typ(&self) -> char {
        self.typ as char
    }

    pub fn value_ptr(&self) -> &GvdbPointer {
        &self.value
    }
}
