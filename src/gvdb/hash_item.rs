use crate::gvdb::error::{GvdbError, GvdbResult};
use crate::gvdb::hash::GvdbHashTable;
use crate::gvdb::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;
use std::fmt::{Display, Formatter};

pub enum GvdbValue<'a> {
    Variant(glib::Variant),
    HashTable(GvdbHashTable<'a>),
}

#[derive(PartialEq)]
pub enum GvdbHashItemType {
    Value,
    HashTable,
    Container,
}

impl From<GvdbHashItemType> for u8 {
    fn from(item: GvdbHashItemType) -> Self {
        match item {
            GvdbHashItemType::Value => 'v' as u8,
            GvdbHashItemType::HashTable => 'H' as u8,
            GvdbHashItemType::Container => 'L' as u8,
        }
    }
}

impl TryFrom<u8> for GvdbHashItemType {
    type Error = GvdbError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let chr = value as char;
        if chr == 'v' {
            Ok(GvdbHashItemType::Value)
        } else if chr == 'H' {
            Ok(GvdbHashItemType::HashTable)
        } else if chr == 'L' {
            Ok(GvdbHashItemType::Container)
        } else {
            Err(GvdbError::InvalidData)
        }
    }
}

impl Display for GvdbHashItemType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            GvdbHashItemType::Value => "Value",
            GvdbHashItemType::HashTable => "HashTable",
            GvdbHashItemType::Container => "Child",
        };

        write!(f, "{}", text)
    }
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
        key_ptr: GvdbPointer,
        typ: GvdbHashItemType,
        value: GvdbPointer,
    ) -> Self {
        let key_start = key_ptr.start();
        let key_size = key_ptr.size() as u16;

        let typ = typ.try_into().unwrap_or('v' as u8);

        Self {
            hash_value: hash_value.to_le(),
            parent: parent.to_le(),
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

    pub fn key_ptr(&self) -> GvdbPointer {
        GvdbPointer::new(
            self.key_start() as usize,
            self.key_start() as usize + self.key_size() as usize,
        )
    }

    pub fn typ(&self) -> GvdbResult<GvdbHashItemType> {
        self.typ.try_into()
    }

    pub fn value_ptr(&self) -> &GvdbPointer {
        &self.value
    }
}
