use std::fmt::{Debug, Formatter};
use crate::gvdb::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;

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

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct GvdbHashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

unsafe impl TriviallyTransmutable for GvdbHashHeader {}

impl GvdbHashHeader {
    pub fn new(n_bloom_words: u32, n_buckets: u32) -> Self {
        Self {
            n_bloom_words,
            n_buckets,
        }
    }

    pub fn n_bloom_words(&self) -> u32 {
        u32::from_le(self.n_bloom_words) & (1 << 27) - 1
    }

    pub fn n_buckets(&self) -> u32 {
        u32::from_le(self.n_buckets)
    }
}

impl Debug for GvdbHashHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GvdbHashHeader {{ n_bloom_words: {}, n_buckets: {} }}", self.n_bloom_words(), self.n_buckets())
    }
}