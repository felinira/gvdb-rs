use crate::read::error::{GvdbReaderError, GvdbReaderResult};
use crate::read::pointer::GvdbPointer;
use safe_transmute::TriviallyTransmutable;
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Eq, Debug)]
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
        let key_start = key_ptr.start().to_le();
        let key_size = (key_ptr.size() as u16).to_le();

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

impl std::fmt::Debug for GvdbHashItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GvdbHashItem")
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
    use crate::read::{GvdbHashItem, GvdbHashItemType, GvdbPointer, GvdbReaderError};
    use matches::assert_matches;

    #[test]
    fn derives() {
        let typ = GvdbHashItemType::Value;
        println!("{}, {:?}", typ, typ);
        let typ = GvdbHashItemType::HashTable;
        println!("{}, {:?}", typ, typ);
        let typ = GvdbHashItemType::Container;
        println!("{}, {:?}", typ, typ);

        let item = GvdbHashItem::new(
            0,
            0,
            GvdbPointer::NULL,
            GvdbHashItemType::Value,
            GvdbPointer::NULL,
        );
        let item2 = item.clone();
        println!("{:?}", item2);
    }

    #[test]
    fn type_try_from() {
        assert_matches!(
            GvdbHashItemType::try_from(b'v'),
            Ok(GvdbHashItemType::Value)
        );
        assert_matches!(
            GvdbHashItemType::try_from(b'H'),
            Ok(GvdbHashItemType::HashTable)
        );
        assert_matches!(
            GvdbHashItemType::try_from(b'L'),
            Ok(GvdbHashItemType::Container)
        );
        assert_matches!(
            GvdbHashItemType::try_from(b'x'),
            Err(GvdbReaderError::InvalidData)
        );
        assert_matches!(
            GvdbHashItemType::try_from(b'?'),
            Err(GvdbReaderError::InvalidData)
        );
    }

    #[test]
    fn item() {
        let item = GvdbHashItem::new(
            0,
            0,
            GvdbPointer::NULL,
            GvdbHashItemType::Value,
            GvdbPointer::NULL,
        );

        assert_eq!(item.hash_value(), 0);
        assert_eq!(item.parent(), 0);
        assert_eq!(item.key_ptr(), GvdbPointer::NULL);
        assert_matches!(item.typ(), Ok(GvdbHashItemType::Value));
        assert_eq!(item.value_ptr(), &GvdbPointer::NULL);
    }
}
