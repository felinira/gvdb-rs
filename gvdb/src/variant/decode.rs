use serde::Deserialize;

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

#[cfg(unix)]
type GVariantDeserializer<'de, 'sig, 'f> =
    zvariant::gvariant::Deserializer<'de, 'sig, 'f, zvariant::Fd<'f>>;
#[cfg(not(unix))]
type GVariantDeserializer<'de, 'sig, 'f> = zvariant::gvariant::Deserializer<'de, 'sig, 'f, ()>;

fn zvariant_get_deserializer(data: &[u8], endian: Endian) -> GVariantDeserializer {
    let context = zvariant::serialized::Context::new_gvariant(endian.into(), 0);

    GVariantDeserializer::new(
        data,
        #[cfg(unix)]
        None::<&[zvariant::Fd]>,
        <zvariant::Value as zvariant::Type>::signature(),
        context,
    )
    .expect("zvariant::Value::signature() must be a valid zvariant signature")
}

impl<'a, T> DecodeVariant<'a> for T
where
    T: zvariant::Type + serde::Deserialize<'a> + 'a,
{
    fn decode(data: &'a [u8], endian: Endian) -> crate::read::Result<Self> {
        let mut de = zvariant_get_deserializer(data, endian);

        serde::Deserialize::deserialize(&mut de).map_err(move |err| {
            crate::read::Error::Data(format!(
                "Error deserializing value as gvariant type \"{}\": {}",
                T::signature(),
                err
            ))
        })
    }
}

impl<'a, T> DecodeVariant<'a> for DecodeValue<'a, T>
where
    T: zvariant::Type + serde::Deserialize<'a> + 'a + DecodeVariant<'a>,
{
    fn decode(data: &'a [u8], endian: Endian) -> crate::read::Result<Self> {
        let mut de = zvariant_get_deserializer(data, endian);

        Ok(DecodeValue::new(
            zvariant::DeserializeValue::deserialize(&mut de)
                .map_err(move |err| {
                    crate::read::Error::Data(format!(
                        "Error deserializing value as gvariant type \"{}\": {}",
                        <T as zvariant::Type>::signature(),
                        err
                    ))
                })?
                .0,
        ))
    }
}
