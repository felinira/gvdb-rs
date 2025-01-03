mod decode;
mod encode;

pub use decode::{DecodeValue, DecodeVariant};
pub use encode::{EncodeValue, EncodeVariant};

use crate::Endian;

/// Types that have a GVariant signature
pub trait VariantType {
    /// The GVariant type string
    fn signature() -> String;
}

impl<T> VariantType for T
where
    T: zvariant::Type,
{
    fn signature() -> String {
        <T as zvariant::Type>::SIGNATURE.to_string()
    }
}
