use crate::read::error::{Error, Result};
use crate::read::pointer::Pointer;
use std::fmt::{Display, Formatter};
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes};

#[derive(PartialEq, Eq, Debug)]
pub enum HashItemType {
    Value,
    HashTable,
    Container,
}

impl From<HashItemType> for u8 {
    fn from(item: HashItemType) -> Self {
        match item {
            HashItemType::Value => b'v',
            HashItemType::HashTable => b'H',
            HashItemType::Container => b'L',
        }
    }
}

impl TryFrom<u8> for HashItemType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        let chr = value as char;
        if chr == 'v' {
            Ok(HashItemType::Value)
        } else if chr == 'H' {
            Ok(HashItemType::HashTable)
        } else if chr == 'L' {
            Ok(HashItemType::Container)
        } else {
            Err(Error::Data(format!("Invalid HashItemType: '{}'", chr)))
        }
    }
}

impl Display for HashItemType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            HashItemType::Value => "Value",
            HashItemType::HashTable => "HashTable",
            HashItemType::Container => "Child",
        };

        write!(f, "{}", text)
    }
}

/// GVDB hash item.
///
/// ```text
/// +-------+----------------------+
/// | Bytes | Field                |
/// +-------+----------------------+
/// |     4 | djb2 hash value      |
/// +-------+----------------------+
/// |     4 | parent item index    |
/// +-------+----------------------+
/// |     4 | start address of key |
/// +-------+----------------------+
/// |     2 | size of key          |
/// +-------+----------------------+
/// |     1 | hash item kind       |
/// +-------+------------------- --+
/// |     1 | unused               |
/// +-------+----------------------+
/// |     8 | value data pointer   |
/// +-------+----------------------+
/// ```
#[repr(C)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct HashItem {
    hash_value: u32,
    parent: u32,

    key_start: u32,
    key_size: u16,

    typ: u8,
    unused: u8,

    value: Pointer,
}

impl HashItem {
    pub fn new(
        hash_value: u32,
        parent: Option<u32>,
        key_ptr: Pointer,
        typ: HashItemType,
        value: Pointer,
    ) -> Self {
        let key_start = key_ptr.start().to_le();
        let key_size = (key_ptr.size() as u16).to_le();
        let parent = if let Some(parent) = parent {
            parent
        } else {
            u32::MAX
        };

        Self {
            hash_value: hash_value.to_le(),
            parent: parent.to_le(),
            key_start,
            key_size,
            typ: typ.into(),
            unused: 0,
            value,
        }
    }

    /// djb hash value of the item data.
    pub fn hash_value(&self) -> u32 {
        u32::from_le(self.hash_value)
    }

    /// The item index of the parent hash item.
    ///
    /// 0xFFFFFFFF means this is a root item.
    pub fn parent(&self) -> Option<u32> {
        let parent = u32::from_le(self.parent);
        if parent == u32::MAX {
            None
        } else {
            Some(parent)
        }
    }

    /// Global start pointer of the key data
    pub fn key_start(&self) -> u32 {
        u32::from_le(self.key_start)
    }

    /// The size of the key data.
    pub fn key_size(&self) -> u16 {
        u16::from_le(self.key_size)
    }

    /// Convenience method to generate a proper GVDB pointer from key_start and key_size.
    pub fn key_ptr(&self) -> Pointer {
        Pointer::new(
            self.key_start() as usize,
            self.key_start() as usize + self.key_size() as usize,
        )
    }

    /// The kind of hash item.
    pub fn typ(&self) -> Result<HashItemType> {
        self.typ.try_into()
    }

    /// A pointer to the underlying data.
    pub fn value_ptr(&self) -> &Pointer {
        &self.value
    }
}

impl std::fmt::Debug for HashItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashItem")
            .field("hash_value", &self.hash_value())
            .field("parent", &self.parent())
            .field("key_start", &self.key_start())
            .field("key_size", &self.key_size())
            .field("typ", &self.typ())
            .field("unused", &self.unused)
            .field("value", &self.value_ptr())
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::read::{Error, HashItem, HashItemType, Pointer};
    use matches::assert_matches;

    #[test]
    fn derives() {
        let typ = HashItemType::Value;
        println!("{}, {:?}", typ, typ);
        let typ = HashItemType::HashTable;
        println!("{}, {:?}", typ, typ);
        let typ = HashItemType::Container;
        println!("{}, {:?}", typ, typ);

        let item = HashItem::new(0, None, Pointer::NULL, HashItemType::Value, Pointer::NULL);
        let item2 = item;
        println!("{:?}", item2);
    }

    #[test]
    fn type_try_from() {
        assert_matches!(HashItemType::try_from(b'v'), Ok(HashItemType::Value));
        assert_matches!(HashItemType::try_from(b'H'), Ok(HashItemType::HashTable));
        assert_matches!(HashItemType::try_from(b'L'), Ok(HashItemType::Container));
        assert_matches!(HashItemType::try_from(b'x'), Err(Error::Data(_)));
        assert_matches!(HashItemType::try_from(b'?'), Err(Error::Data(_)));
    }

    #[test]
    fn item() {
        let item = HashItem::new(
            0,
            Some(0),
            Pointer::NULL,
            HashItemType::Value,
            Pointer::NULL,
        );

        assert_eq!(item.hash_value(), 0);
        assert_eq!(item.parent(), Some(0));
        assert_eq!(item.key_ptr(), Pointer::NULL);
        assert_matches!(item.typ(), Ok(HashItemType::Value));
        assert_eq!(item.value_ptr(), &Pointer::NULL);
    }
}
