pub mod error;
pub mod hash;
pub mod header;
pub mod pointer;
pub mod reader;
pub mod table;

#[cfg(test)]
mod test;
mod util;

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

type Link<T> = Option<Rc<RefCell<T>>>;

pub enum GvdbItemValue {
    Value(glib_sys::GVariant),
    Table(HashMap<String, Box<dyn Any>>),
    Child(Link<GvdbItem>),
    None,
}

pub struct GvdbItem {
    key: String,
    hash_value: u32,
    // Little endian
    assigned_index: u32,
    parent: Link<GvdbItem>,
    sibling: Link<GvdbItem>,
    next: Link<GvdbItem>,

    value: GvdbItemValue,
}

pub struct GvdbBuilder {
    chunks: VecDeque<GvdbChunk>,
    offset: usize,
    byteswap: bool,
}

pub struct GvdbChunk {
    offset: usize,
    data: Vec<u8>,
}

impl GvdbBuilder {
    pub fn new(byteswap: bool) -> Self {
        Self {
            chunks: Default::default(),
            offset: 0,
            byteswap,
        }
    }
}
