use crate::gvdb::error::{GvdbError, GvdbResult};
use deku::DekuContainerRead;
use std::array::TryFromSliceError;
use std::mem::size_of;

pub trait ReadFromBytes<'a, T> {
    fn from_bytes_exact(bytes: &'a [u8]) -> GvdbResult<T>;
    fn from_bytes_aligned(bytes: &'a [u8], alignment: usize) -> GvdbResult<(&'a [u8], T)>;
    fn from_bytes_aligned_exact(bytes: &'a [u8], alignment: usize) -> GvdbResult<T>;
}

impl<'a, T> ReadFromBytes<'a, T> for T
where
    T: Sized + deku::DekuContainerRead<'a>,
{
    fn from_bytes_exact(bytes: &'a [u8]) -> GvdbResult<T> {
        let ((rest, bitoffset), this) = DekuContainerRead::from_bytes((bytes, 0))?;

        if !rest.is_empty() || bitoffset != 0 {
            Err(GvdbError::InvalidData)
        } else {
            Ok(this)
        }
    }

    fn from_bytes_aligned(bytes: &'a [u8], alignment: usize) -> GvdbResult<(&'a [u8], T)> {
        let ((rest, bitoffset), this) = DekuContainerRead::from_bytes((bytes, 0))?;

        if bitoffset != 0 {
            Err(GvdbError::InvalidData)
        } else {
            // Align to nearest alignment
            let size = bytes.len() - rest.len();
            let offset = size % alignment;
            let rest = rest.get(offset..).ok_or(GvdbError::DataAlignment)?;
            Ok((rest, this))
        }
    }

    fn from_bytes_aligned_exact(bytes: &'a [u8], alignment: usize) -> GvdbResult<T> {
        let (rest, result) = Self::from_bytes_aligned(bytes, alignment)?;

        if rest.is_empty() {
            Ok(result)
        } else {
            Err(GvdbError::InvalidData)
        }
    }
}

pub struct TryFromByteSliceError();

impl From<TryFromSliceError> for TryFromByteSliceError {
    fn from(_: TryFromSliceError) -> Self {
        Self()
    }
}

pub trait TryFromByteSlice<T> {
    fn try_from_byte_slice<F>(slice: &[u8], converter: F) -> Result<Vec<T>, TryFromByteSliceError>
    where
        F: Fn(&[u8]) -> Result<T, TryFromByteSliceError> + Sized,
    {
        let mut iter = slice.chunks_exact(size_of::<u32>());
        let mut res = Vec::with_capacity(slice.len() / size_of::<u32>());
        while let Some(data) = iter.next() {
            res.push(converter(data)?);
        }

        if iter.remainder().is_empty() {
            Ok(res)
        } else {
            Err(TryFromByteSliceError())
        }
    }

    fn try_from_le_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized;

    fn try_from_be_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized;

    fn try_from_ne_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized;
}

impl TryFromByteSlice<u32> for Vec<u32> {
    fn try_from_le_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized,
    {
        Self::try_from_byte_slice(slice, |data| Ok(u32::from_le_bytes(data.try_into()?)))
    }

    fn try_from_be_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized,
    {
        Self::try_from_byte_slice(slice, |data| Ok(u32::from_be_bytes(data.try_into()?)))
    }

    fn try_from_ne_byte_slice(slice: &[u8]) -> Result<Self, TryFromByteSliceError>
    where
        Self: Sized,
    {
        Self::try_from_byte_slice(slice, |data| Ok(u32::from_ne_bytes(data.try_into()?)))
    }
}

/// Perform the djb2 hash function
pub fn djb_hash(key: &str) -> u32 {
    let mut hash_value: u32 = 5381;
    for char in key.bytes() {
        hash_value = hash_value.wrapping_mul(33).wrapping_add(char as u32);
    }

    hash_value
}
