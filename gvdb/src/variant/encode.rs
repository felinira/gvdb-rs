use super::{Endian, VariantType};

/// Helper struct used to encode a gvariant Variant internally
pub struct EncodeValue<T: ?Sized>(
    /// The inner decoded value
    pub T,
);

impl<T> EncodeValue<T> {
    /// Create a new EncodeValue to wrap in a GVariant VARIANT type
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

/// Types that can be encoded to GVariant data
///
/// This trait is internal to gvdb and only documented publically for informational purposes.
///
/// If you are missing an item here, feel free open an issue.
pub trait EncodeVariant<'a> {
    /// Encode the type from the specified data for the target endianness
    fn encode(&self, endian: Endian) -> crate::write::Result<Box<[u8]>>;
}

impl<'a> std::fmt::Debug for dyn EncodeVariant<'a> + 'a {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("dyn EncodeVariant").finish()
    }
}

impl<'a, T> VariantType for EncodeValue<T>
where
    T: zvariant::Type + EncodeVariant<'a>,
{
    fn signature() -> String {
        <zvariant::Value as zvariant::Type>::signature().to_string()
    }
}

impl<'a, T> EncodeVariant<'a> for T
where
    T: zvariant::Type + serde::Serialize + 'a,
{
    fn encode(&self, endian: Endian) -> crate::write::Result<Box<[u8]>> {
        let context = zvariant::serialized::Context::new_gvariant(endian.into(), 0);
        Ok(Box::from(&*zvariant::to_bytes(context, self)?))
    }
}

impl<T> EncodeVariant<'_> for EncodeValue<T>
where
    T: zvariant::Type,
    T: serde::Serialize + std::fmt::Debug,
{
    fn encode(&self, endian: Endian) -> crate::write::Result<Box<[u8]>> {
        let context = zvariant::serialized::Context::new_gvariant(endian.into(), 0);
        Ok(Box::from(&*zvariant::to_bytes(
            context,
            &zvariant::SerializeValue(&self.0),
        )?))
    }
}

impl<'a, T: std::fmt::Debug> std::fmt::Debug for EncodeValue<T>
where
    T: EncodeVariant<'a>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValueWrapper").field(&self.0).finish()
    }
}
