use crate::read::error::{GvdbReaderError, GvdbReaderResult};
use crate::read::hash::GvdbHashTable;
use crate::read::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;
use std::fmt::{Display, Formatter};

#[cfg(not(feature = "glib"))]
use crate::no_glib::Variant;
#[cfg(feature = "glib")]
use glib::Variant;

pub enum GvdbValue<'a> {
    Variant(Variant),
    HashTable(GvdbHashTable<'a>),
}

#[derive(PartialEq, Debug)]
pub enum GvdbHashItemType {
    Value,
    HashTable,
    Container,
}

impl From<GvdbHashItemType> for u8 {
    fn from(item: GvdbHashItemType) -> Self {
        match item {
            GvdbHashItemType::Value => b'v',
            GvdbHashItemType::HashTable => b'H',
            GvdbHashItemType::Container => b'L',
        }
    }
}

impl TryFrom<u8> for GvdbHashItemType {
    type Error = GvdbReaderError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let chr = value as char;
        if chr == 'v' {
            Ok(GvdbHashItemType::Value)
        } else if chr == 'H' {
            Ok(GvdbHashItemType::HashTable)
        } else if chr == 'L' {
            Ok(GvdbHashItemType::Container)
        } else {
            Err(GvdbReaderError::InvalidData)
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

        let typ = typ.try_into().unwrap_or(b'v');

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

    pub fn typ(&self) -> GvdbReaderResult<GvdbHashItemType> {
        self.typ.try_into()
    }

    pub fn value_ptr(&self) -> &GvdbPointer {
        &self.value
    }
}
