use zvariant::DynamicType;

use super::Endian;

/// Helper struct used to decode a gvariant Variant internally
pub struct DecodeValue<'a, T>(
    /// The inner decoded value
    pub T,
    /// Marker
    std::marker::PhantomData<&'a T>,
);

impl<T> DecodeValue<'_, T> {
    /// Create a new DecodeValue to unwrap from a GVariant VARIANT type
    pub fn new(value: T) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

/// Types that can be decoded from GVariant data
///
/// This trait is internal to gvdb and only documented publically for informational purposes.
///
/// If you are missing an item here, feel free open an issue.
pub trait DecodeVariant<'a>
where
    Self: Sized,
{
    /// Decode the type from the specified data for the target endianness
    fn decode(data: &'a [u8], endian: Endian) -> crate::read::Result<Self>;
}

fn zvariant_deserialize(data: &[u8], endian: Endian) -> crate::read::Result<zvariant::OwnedValue> {
    // This always allocates, but it's the best we can do with this API
    let context = zvariant::serialized::Context::new_gvariant(endian.into(), 0);
    let data = zvariant::serialized::Data::new(data, context);
    let value: zvariant::OwnedValue = data.deserialize()?.0;
    Ok(value)
}

impl<'a, T> DecodeVariant<'a> for T
where
    T: zvariant::Type + serde::Deserialize<'a> + 'a + TryFrom<zvariant::OwnedValue>,
    <T as TryFrom<zvariant::OwnedValue>>::Error: Into<zvariant::Error>,
{
    fn decode(data: &'a [u8], endian: Endian) -> crate::read::Result<Self> {
        let value = zvariant_deserialize(data, endian)?;
        let signature = value.signature();
        value.try_into().map_err(|_e| {
            crate::read::Error::Data(format!(
                "Error interpreting gvariant data with signature {} as {}",
                signature,
                T::SIGNATURE,
            ))
        })
    }
}

impl<'a, T> DecodeVariant<'a> for DecodeValue<'a, T>
where
    T: zvariant::Type
        + serde::Deserialize<'a>
        + 'a
        + DecodeVariant<'a>
        + TryFrom<zvariant::OwnedValue>,
    <T as TryFrom<zvariant::OwnedValue>>::Error: Into<zvariant::Error>,
{
    fn decode(data: &'a [u8], endian: Endian) -> crate::read::Result<Self> {
        let value = zvariant_deserialize(data, endian)?;
        let inner: zvariant::OwnedValue =
            value.downcast_ref::<zvariant::Value>()?.try_to_owned()?;
        let signature = value.signature();
        Ok(DecodeValue::new(inner.try_into().map_err(|_e| {
            crate::read::Error::Data(format!(
                "Error interpreting gvariant data with signature {} as {}",
                signature,
                T::SIGNATURE,
            ))
        })?))
    }
}
