use std::fmt::{Debug, Formatter};
use crate::gvdb::pointer::GvdbPointer;
use deku::prelude::*;

#[repr(C)]
#[derive(DekuRead, DekuWrite)]
pub struct GvdbHashItem {
    #[deku(endian = "little")]
    hash_value: u32,
    #[deku(endian = "little")]
    parent: u32,

    #[deku(endian = "little")]
    key_start: u32,
    #[deku(endian = "little")]
    key_size: u16,

    #[deku(endian = "little")]
    typ: u8,
    #[deku(endian = "little")]
    unused: u8,

    // no endianness here otherwise we would need context
    value: GvdbPointer,
}

impl GvdbHashItem {
    pub fn hash_value(&self) -> u32 {
        self.hash_value
    }

    pub fn parent(&self) -> u32 {
        self.parent
    }

    pub fn key_start(&self) -> u32 {
        self.key_start
    }

    pub fn key_size(&self) -> u16 {
        self.key_size
    }

    pub fn typ(&self) -> char {
        self.typ as char
    }

    pub fn value_ptr(&self) -> &GvdbPointer {
        &self.value
    }
}

#[repr(C)]
#[derive(PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct GvdbHashHeader {
    n_bloom_words: u32,
    n_buckets: u32,
}

impl GvdbHashHeader {
    pub fn new(n_bloom_words: u32, n_buckets: u32) -> Self {
        Self {
            n_bloom_words,
            n_buckets,
        }
    }

    pub fn n_bloom_words(&self) -> u32 {
        self.n_bloom_words & (1 << 27) - 1
    }

    pub fn n_buckets(&self) -> u32 {
        self.n_buckets
    }
}

impl Debug for GvdbHashHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GvdbHashHeader {{ n_bloom_words: {}, n_buckets: {} }}", self.n_bloom_words(), self.n_buckets())
    }
}