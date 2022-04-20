use super::variant::{FromVariant, ToVariant, Variant};
use super::variant_type::VariantTy;
use std::borrow::Cow;

impl ToVariant for bool {
    fn to_variant(&self) -> Variant {
        let byte: u8 = if *self { 1 } else { 0 };
        Variant::from_data_with_type(byte.to_le_bytes(), VariantTy::BOOLEAN)
    }
}

impl ToVariant for u8 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::BYTE)
    }
}

impl ToVariant for i16 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::INT16)
    }
}
impl ToVariant for u16 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::UINT16)
    }
}

impl ToVariant for i32 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::INT32)
    }
}

impl ToVariant for u32 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::UINT32)
    }
}

impl ToVariant for i64 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::INT64)
    }
}

impl ToVariant for u64 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::UINT64)
    }
}

impl ToVariant for f64 {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.to_le_bytes(), VariantTy::DOUBLE)
    }
}

impl ToVariant for &[u8] {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self, VariantTy::BYTE_STRING)
    }
}

impl ToVariant for dyn AsRef<[u8]> {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self.as_ref(), VariantTy::BYTE_STRING)
    }
}

impl ToVariant for Cow<'_, [u8]> {
    fn to_variant(&self) -> Variant {
        Variant::from_data_with_type(self, VariantTy::BYTE_STRING)
    }
}

impl ToVariant for &str {
    fn to_variant(&self) -> Variant {
        Variant::new_string(self.to_string())
    }
}

impl FromVariant for bool {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::BOOLEAN {
            let value = u8::from_le_bytes(variant.data().try_into().ok()?);
            Some(value != 0)
        } else {
            None
        }
    }
}

impl FromVariant for u8 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::BYTE {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for i16 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::INT16 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for u16 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::UINT16 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for i32 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::INT32 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for u32 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::UINT32 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for i64 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::INT64 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for u64 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::UINT64 {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for f64 {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::DOUBLE {
            Some(Self::from_le_bytes(variant.data().try_into().ok()?))
        } else {
            None
        }
    }
}

impl FromVariant for String {
    fn from_variant(variant: &Variant) -> Option<Self> {
        if variant.type_() == VariantTy::STRING {
            variant.str().map(ToString::to_string)
        } else {
            None
        }
    }
}
