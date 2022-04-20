use super::variant_type::*;
use std::ffi::{CStr, CString};
use std::fmt::{Debug, Display, Formatter};

/// A reimplementation of [`struct@glib::Variant`]
#[derive(PartialEq)]
pub struct Variant {
    typ: VariantType,
    data: Vec<u8>,
    type_info: TypeInfo,
}

impl Variant {
    fn from_data_with_type_info<A: AsRef<[u8]>>(
        data: A,
        typ: &VariantTy,
        type_info: TypeInfo,
    ) -> Self {
        let typ = typ.to_owned();

        Self {
            typ: typ.to_owned(),
            data: data.as_ref().to_vec(),
            type_info,
        }
    }

    /// Return a `Variant` with the provided (unchecked) data and the specified type
    pub fn from_data_with_type<A: AsRef<[u8]>>(data: A, typ: &VariantTy) -> Self {
        let type_info = typ.type_info();
        Self::from_data_with_type_info(data, typ, type_info)
    }

    /// Return a new `Variant` with type string and the specified string as data
    pub fn new_string(string: String) -> Self {
        let typ = VariantTy::STRING;
        let data = CString::new(string).unwrap().as_bytes_with_nul().to_vec();
        Self::from_data_with_type(data, typ)
    }

    /// Uses [`FromVariant`][crate::no_glib::FromVariant] to extract a native value
    pub fn get<T: FromVariant>(&self) -> Option<T> {
        T::from_variant(self)
    }

    /// Return the [`VariantTy`][crate::no_glib::VariantTy] corresponding to this Variant
    pub fn type_(&self) -> &VariantTy {
        &self.typ
    }

    /// Return whether this type is the same or a subtype of other
    pub fn is_type(&self, other: &VariantTy) -> bool {
        self.typ.is_subtype_of(other)
    }

    /// Return whether the corresponding type is a basic (non-container) type
    pub fn is_basic(&self) -> bool {
        self.typ.is_basic()
    }

    /// Return whether the corresponding type is a container type
    pub fn is_container(&self) -> bool {
        self.typ.is_container()
    }

    /// Return whether the corresponding type is a maybe type
    pub fn is_maybe(&self) -> bool {
        self.typ.is_maybe()
    }

    /// Return whether the corresponding type is an array
    pub fn is_array(&self) -> bool {
        self.typ.is_array()
    }

    /// Return whether the corresponding type is a tuple
    pub fn is_tuple(&self) -> bool {
        self.typ.is_tuple()
    }

    /// Return whether the corresponding type is a dict entry
    pub fn is_dict_entry(&self) -> bool {
        self.typ.is_dict_entry()
    }

    /// Return whether the corresponding type is a [`VARIANT`][crate::no_glib::VariantTy::VARIANT] type
    pub fn is_variant(&self) -> bool {
        self.typ.is_variant()
    }

    fn varsize_container_offset_item_size(&self) -> usize {
        let array_len = self.data.len() as u64;
        if array_len > u32::MAX as u64 {
            8
        } else if array_len > u16::MAX as u64 {
            4
        } else if array_len > u8::MAX as u64 {
            2
        } else if array_len > 0 {
            1
        } else {
            0
        }
    }

    fn varsize_container_offset_size_for_n(data_len: usize, n_varsize_items: usize) -> usize {
        let data_len = data_len as u64;
        let n_items = n_varsize_items as u64;

        if data_len + 8 * n_items > u32::MAX as u64 {
            8
        } else if data_len + 4 * n_items > u16::MAX as u64 {
            4
        } else if data_len + 2 * n_items > u8::MAX as u64 {
            2
        } else {
            1
        }
    }

    fn read_unaligned_le(data: &[u8], size: usize) -> usize {
        let aligned_copy = data[0..size].to_vec();

        if size == 8 {
            u64::from_le_bytes(aligned_copy.try_into().unwrap()) as usize
        } else if size == 4 {
            u32::from_le_bytes(aligned_copy.try_into().unwrap()) as usize
        } else if size == 2 {
            u16::from_le_bytes(aligned_copy.try_into().unwrap()) as usize
        } else {
            u8::from_le_bytes(aligned_copy.try_into().unwrap()) as usize
        }
    }

    fn add_offsets_to_varsize_container_data(data: &mut Vec<u8>, offsets: Vec<usize>) {
        let offsets_size = Self::varsize_container_offset_size_for_n(data.len(), offsets.len());
        for offset in offsets {
            if offsets_size == 8 {
                data.extend_from_slice(&(offset as u64).to_le_bytes())
            } else if offsets_size == 4 {
                data.extend_from_slice(&(offset as u32).to_le_bytes())
            } else if offsets_size == 2 {
                data.extend_from_slice(&(offset as u16).to_le_bytes())
            } else {
                data.extend_from_slice(&(offset as u8).to_le_bytes())
            };
        }
    }

    /// Create a new array `Variant` from the provided iterator
    pub fn array_from_iter_with_type<T: AsRef<Variant>, I: IntoIterator<Item = T>>(
        typ: &VariantTy,
        children: I,
    ) -> Self {
        let mut data = Vec::new();
        let type_info = typ.type_info();
        let mut offsets = Vec::new();

        for child in children {
            if &*child.as_ref().typ != typ {
                panic!("Tried to create array with different type than specified");
            }

            data.extend_from_slice(&child.as_ref().data);

            if type_info.fixed_size == 0 {
                offsets.push(data.len());
            }
        }

        Self::add_offsets_to_varsize_container_data(&mut data, offsets);
        Self::from_data_with_type(data, &VariantType::new_array(typ).unwrap())
    }

    /// Create a new tuple `Variant` from the provided iterator
    pub fn tuple_from_iter(variants: impl IntoIterator<Item = impl AsRef<Variant>>) -> Self {
        let mut offsets = Vec::new();

        let mut types = vec![b'('];
        let mut data = Vec::new();

        let mut iter = variants.into_iter().peekable();
        let is_empty = iter.peek().is_none();

        while let Some(variant) = iter.next() {
            let variant = variant.as_ref();

            types.extend_from_slice(variant.typ.as_bytes());
            let typeinfo = variant.typ.type_info();
            while data.len() % typeinfo.alignment as usize != 0 {
                data.push(0);
            }

            data.append(&mut variant.data.clone());

            // variable length type offset, but not for the last element
            if typeinfo.fixed_size == 0 && iter.peek().is_some() {
                offsets.push(data.len());
            }
        }

        types.push(b')');

        if is_empty {
            data.push(0);
        }

        let typ = unsafe { VariantTy::from_slice_unchecked(&types) };

        // for some reason tuple offsets are the other way
        offsets.reverse();
        Self::add_offsets_to_varsize_container_data(&mut data, offsets);
        Self::from_data_with_type(data, typ)
    }

    /// Create a new [`VARIANT`][crate::no_glib::VariantTy::VARIANT] type from the provided `Variant`
    pub fn from_variant(variant: &Variant) -> Self {
        let typ = VariantTy::VARIANT;
        let type_str = CString::new(variant.typ.as_str()).unwrap();

        let mut data = variant.data().to_vec();
        data.push(0);
        data.extend_from_slice(type_str.as_bytes());

        Self::from_data_with_type(data, typ)
    }

    /// Return the raw data corresponding to this `Variant`
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Return a copy of the raw data corresponding to this `Variant`
    pub fn data_as_bytes(&self) -> Box<[u8]> {
        self.data().to_vec().into_boxed_slice()
    }

    /// Swaps the endianness of this `Variant`
    pub fn byteswap(&self) -> Self {
        todo!()
    }

    /// Provided only for compatibility. This implementation always uses normal form for all data.
    pub fn normal_form(&self) -> &Self {
        self
    }

    fn variable_length_array_offsets_size(&self) -> usize {
        let offset_size = self.varsize_container_offset_item_size();
        let last_end =
            Self::read_unaligned_le(&self.data[self.data.len() - offset_size..], offset_size);

        if last_end > self.data.len() {
            return 0;
        }

        let offsets_array_size = self.data.len() - last_end;
        if offsets_array_size % offset_size != 0 {
            0
        } else {
            offsets_array_size
        }
    }

    /// Return the number of children in this `Variant`
    pub fn n_children(&self) -> usize {
        let type_info = self.typ.type_info();
        let value_size = self.data.len();

        if self.typ.is_maybe() {
            if value_size > 0 {
                1
            } else {
                0
            }
        } else if self.typ.is_array() {
            let element_fixed_size = type_info.element_fixed_size;
            if element_fixed_size != 0 && value_size % element_fixed_size == 0 {
                value_size / element_fixed_size
            } else {
                let offsets_array_size = self.variable_length_array_offsets_size();
                if offsets_array_size == 0 {
                    // invalid data
                    0
                } else {
                    offsets_array_size / self.varsize_container_offset_item_size()
                }
            }
        } else if self.typ.is_tuple() || self.typ.is_variant() {
            type_info.n_members
        } else {
            0
        }
    }

    /// Return the child value at `index`
    pub fn try_child_value(&self, index: usize) -> Option<Self> {
        let n_children = self.n_children();

        if n_children <= index {
            None
        } else if self.is_array() {
            let typ = self.typ.type_element();
            let elem_size = typ.type_info().fixed_size;
            if elem_size != 0 {
                // Simple case, we can just index into the array
                let offset = index * elem_size;
                let data = self.data.get(offset..offset + elem_size)?;
                Some(Self::from_data_with_type(data, typ))
            } else {
                let offset_item_size = self.varsize_container_offset_item_size();
                let offsets_start = self.data.len() - self.variable_length_array_offsets_size();
                let child_bucket_offset = offsets_start + index * offset_item_size;
                let child_offset_end = Self::read_unaligned_le(
                    self.data
                        .get(child_bucket_offset..child_bucket_offset + offset_item_size)?,
                    offset_item_size,
                );

                let child_offset_start = if index == 0 {
                    0
                } else {
                    // The end of the previous item + 1 is the start of the new one
                    Self::read_unaligned_le(
                        self.data
                            .get(child_bucket_offset - offset_item_size..child_bucket_offset)?,
                        offset_item_size,
                    )
                };

                let child_data = self.data.get(child_offset_start..child_offset_end)?;
                let child_type = self.typ.type_element();
                Some(Self::from_data_with_type(child_data, child_type))
            }
        } else if self.is_maybe() {
            // Just return self as the value, we checked above if we have a value at all
            let typ = self.typ.type_element();
            Some(Self::from_data_with_type(&self.data, typ))
        } else if self.is_tuple() {
            let tuple_type_info = self.type_info.tuple_type_info.as_ref()?;
            let offset_item_size = self.varsize_container_offset_item_size();
            let offsets_start =
                self.data.len() - offset_item_size * (tuple_type_info.n_unsized_members - 1);
            let offsets_end = self.data.len();
            let mut n_unsized_member = 0;
            let mut offset = 0;
            let mut offset_end = 0;
            let mut typ = VariantTy::ANY;

            for (idx, member_type_info) in tuple_type_info.member_type_info.iter().enumerate() {
                typ = &tuple_type_info.member_types[idx];
                let alignment = member_type_info.alignment as usize;

                if offset % alignment != 0 {
                    offset += alignment - offset % alignment;
                }

                let mut member_size = member_type_info.fixed_size;
                if member_size == 0 {
                    // get size manually
                    if idx == tuple_type_info.member_type_info.len() - 1 {
                        // if this is the last tuple item there is no size information: we will take
                        // the rest of our len as the size
                        member_size = offsets_start - offset;
                    } else {
                        // there is an offsets table at the end of this tuple
                        let member_offset_pos =
                            offsets_end - offset_item_size * (n_unsized_member + 1);
                        member_size = Self::read_unaligned_le(
                            self.data
                                .get(member_offset_pos..member_offset_pos + offset_item_size)?,
                            offset_item_size,
                        ) - offset;
                    }

                    n_unsized_member += 1;
                }

                if index == idx {
                    offset_end = offset + member_size;
                    break;
                }

                offset += member_size;
            }

            let data = if offset_end != 0 {
                self.data.get(offset..offset_end)
            } else {
                self.data.get(offset..)
            }?;

            Some(Self::from_data_with_type(data, typ))
        } else if self.is_variant() {
            // find 0 byte that separates type info from data
            let mut type_info_offset = 0;
            for (index, byte) in self.data.iter().rev().enumerate() {
                if index > 0 && *byte == 0 {
                    type_info_offset = self.data.len() - index;
                    break;
                }
            }

            if type_info_offset != 0 {
                let data = &self.data[..type_info_offset - 1];
                let type_str_data = &self.data[type_info_offset..];
                let typ = VariantTy::new(std::str::from_utf8(type_str_data).ok()?).ok()?;
                Some(Self::from_data_with_type(data, typ))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Return the child value at `index`.
    ///
    /// # Panics
    ///
    /// This function will panic if the index is too large or the contained data is invalid
    pub fn child_value(&self, index: usize) -> Self {
        self.try_child_value(index).unwrap()
    }

    /// Iterate over the child values of this Variant.
    ///
    /// # Panics
    ///
    /// This will panic if the provided `Variant` is not a container
    pub fn iter(&self) -> VariantIter {
        VariantIter::new(self)
    }

    /// Return an `Option<&str>` if this `Variant` is of type String, otherwise return None
    pub fn str(&self) -> Option<&str> {
        if self.typ.is_subtype_of(VariantTy::STRING) {
            if self.data.is_empty() {
                Some("")
            } else {
                let str = CStr::from_bytes_with_nul(self.data()).ok()?.to_str().ok()?;
                Some(str)
            }
        } else {
            None
        }
    }
}

/// An `Iterator` over the children of a container [`Variant`]
pub struct VariantIter<'a> {
    elem: &'a Variant,
    child: usize,
}

impl<'a> VariantIter<'a> {
    /// Create a new `VariantIter`
    pub fn new(elem: &'a Variant) -> Self {
        assert!(elem.typ.is_container());
        Self { elem, child: 0 }
    }
}

impl<'a> Iterator for VariantIter<'a> {
    type Item = Variant;

    fn next(&mut self) -> Option<Self::Item> {
        let child = self.elem.try_child_value(self.child);
        self.child += 1;
        child
    }
}

impl Debug for Variant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_basic() {
            write!(f, "{}", self)
        } else {
            write!(f, "Variant {{ type: {:?}, data: {{ ", self.typ)?;

            if self.is_container() {
                if self.is_tuple() {
                    write!(f, "(")?;
                } else if self.is_array() {
                    write!(f, "[")?;
                }

                for (idx, elem) in self.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }

                    write!(f, "{:?}", elem)?;
                }

                if self.is_tuple() {
                    write!(f, " )")?;
                } else if self.is_array() {
                    write!(f, "]")?;
                }
            } else {
                write!(f, "{:?}", self.data)?;
            }

            write!(f, " }}")
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_basic() {
            let fb = "?".to_string();
            match self.typ.0[0] {
                VariantTy::CLASS_BOOLEAN => {
                    write!(f, "{}", self.get::<bool>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_BYTE => {
                    write!(f, "{}", self.get::<u8>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_INT16 => {
                    write!(f, "{}", self.get::<i16>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_UINT16 => {
                    write!(f, "{}", self.get::<u16>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_INT32 => {
                    write!(f, "{}", self.get::<i32>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_UINT32 => {
                    write!(f, "{}", self.get::<u32>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_INT64 => {
                    write!(f, "{}", self.get::<i64>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_UINT64 => {
                    write!(f, "{}", self.get::<u64>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_DOUBLE => {
                    write!(f, "{}", self.get::<f64>().map_or(fb, |v| v.to_string()))
                }
                VariantTy::CLASS_STRING => {
                    write!(f, "{}", self.get::<String>().map_or(fb, |v| v.to_string()))
                }
                _ => write!(f, "{:?}", self.data),
            }
        } else if self.is_container() {
            if self.is_tuple() {
                write!(f, "(")?;
            } else if self.is_variant() {
                write!(f, "Variant ( ")?;
            } else if self.is_maybe() {
                write!(f, "Maybe (")?;
            } else {
                write!(f, "[")?;
            }

            for (idx, elem) in self.iter().enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }

                if self.is_tuple() {
                    write!(f, "{:?} ", elem.type_())?;
                }

                write!(f, "{}", elem)?;
            }

            if self.is_tuple() || self.is_variant() || self.is_maybe() {
                write!(f, " )")
            } else {
                write!(f, "]")
            }
        } else {
            write!(f, "{:?}", self.data)
        }
    }
}

impl AsRef<Variant> for Variant {
    fn as_ref(&self) -> &Variant {
        self
    }
}

/// Conversion Trait for types to [`Variant`]
pub trait ToVariant {
    /// Returns a `Variant` clone of `self`.
    fn to_variant(&self) -> Variant;
}

#[cfg(feature = "glib")]
impl PartialEq<glib::Variant> for Variant {
    fn eq(&self, other: &glib::Variant) -> bool {
        let other = other.normal_form();
        self.data == other.data() && self.type_() == other.type_()
    }
}

#[cfg(feature = "glib")]
impl PartialEq<Variant> for glib::Variant {
    fn eq(&self, other: &Variant) -> bool {
        other.eq(self)
    }
}

/// Conversion Trait for types from [`Variant`]
pub trait FromVariant: Sized {
    /// Convert this type to a `Variant`
    fn from_variant(variant: &Variant) -> Option<Self>;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tuple() {
        let num1: u8 = 8;
        let num2: u32 = 42;
        let bytes1: &[u8; 5] = &[1, 2, 3, 4, 5];
        let bytes2: &[u8; 5] = &[10, 9, 8, 7, 6];

        let my_num1 = num1.to_variant();
        let my_num2 = num2.to_variant();
        let my_bytes1 = Variant::from_data_with_type(bytes1.to_vec(), VariantTy::BYTE_STRING);
        let my_bytes2 = Variant::from_data_with_type(bytes2.to_vec(), VariantTy::BYTE_STRING);
        let my_tuple = Variant::tuple_from_iter(&[my_num1, my_num2, my_bytes1, my_bytes2]);

        assert_eq!(my_tuple.n_children(), 4);
        assert_eq!(my_tuple.child_value(0), num1.to_variant());
        assert_eq!(my_tuple.child_value(1), num2.to_variant());
        assert_eq!(
            my_tuple.child_value(2),
            Variant::from_data_with_type(bytes1.to_vec(), VariantTy::BYTE_STRING)
        );
        assert_eq!(
            my_tuple.child_value(3),
            Variant::from_data_with_type(bytes2.to_vec(), VariantTy::BYTE_STRING)
        );
    }
}

#[cfg(test)]
#[cfg(feature = "glib")]
mod test_with_glib {
    use super::{Variant, VariantTy};
    use crate::test::assert_bytes_eq;

    #[test]
    fn gvariant_string() {
        let ref_variant = glib::ToVariant::to_variant("test_string").normal_form();
        let ref_data = ref_variant.data();
        let my_variant = Variant::new_string("test_string".to_string());
        let my_data = my_variant.data();
        assert_bytes_eq(ref_data, &my_data, "test");
    }

    #[test]
    fn gvariant_byte_array() {
        let bytes: &[u8; 5] = &[1, 2, 3, 4, 5];

        let ref_variant = glib::ToVariant::to_variant(bytes.as_slice()).normal_form();
        let ref_data = ref_variant.data();
        let my_variant = Variant::from_data_with_type(bytes.to_vec(), VariantTy::BYTE_STRING);
        let my_data = my_variant.data();
        assert_bytes_eq(ref_data, &my_data, "test");
    }

    #[test]
    fn gvariant_u8() {
        let num: u8 = 42;
        let ref_num = glib::ToVariant::to_variant(&num).normal_form();
        let ref_data = ref_num.data();

        let my_variant = super::ToVariant::to_variant(&num);
        let my_data = my_variant.data();
        assert_bytes_eq(ref_data, &my_data, "test");
    }

    #[test]
    fn gvariant_u32() {
        let num: u32 = 42;
        let ref_num = glib::ToVariant::to_variant(&num).normal_form();
        let ref_data = ref_num.data();

        let my_variant = super::ToVariant::to_variant(&num);
        let my_data = my_variant.data();
        assert_bytes_eq(ref_data, &my_data, "test");
    }

    #[test]
    fn gvariant_fixed_array() {
        let bytes: &[u32; 5] = &[19999, 29999, 39999, 49999, 59999];

        let ref_slice = bytes.map(|b| glib::ToVariant::to_variant(&b));
        let ref_array =
            glib::Variant::array_from_iter_with_type(glib::VariantTy::UINT32, ref_slice)
                .normal_form();
        let ref_data = ref_array.data();

        let my_slice = bytes.map(|b| super::ToVariant::to_variant(&b));
        let my_array = Variant::array_from_iter_with_type(VariantTy::UINT32, my_slice);
        let my_data = my_array.data();
        assert_bytes_eq(ref_data, &my_data, "test");

        assert_eq!(ref_array.n_children(), my_array.n_children());
        assert_eq!(ref_array.child_value(0), my_array.child_value(0));
        assert_eq!(ref_array.child_value(1), my_array.child_value(1));
        assert_eq!(ref_array.child_value(2), my_array.child_value(2));
        assert_eq!(ref_array.child_value(3), my_array.child_value(3));
        assert_eq!(ref_array.child_value(4), my_array.child_value(4));
    }

    #[test]
    fn gvariant_variable_array() {
        let strings: &[&str; 3] = &["abc", "test", "123"];

        let ref_slice = strings.map(|b| glib::ToVariant::to_variant(&b));
        let ref_array =
            glib::Variant::array_from_iter_with_type(glib::VariantTy::STRING, ref_slice)
                .normal_form();
        let ref_data = ref_array.data();

        let my_slice = strings.map(|b| super::ToVariant::to_variant(&b));
        let my_array = Variant::array_from_iter_with_type(VariantTy::STRING, my_slice);
        let my_data = my_array.data();
        assert_bytes_eq(ref_data, &my_data, "test");
        assert_eq!(my_array.n_children(), 3);

        assert_eq!(ref_array.child_value(0), my_array.child_value(0));
        assert_eq!(ref_array.child_value(1), my_array.child_value(1));
        assert_eq!(ref_array.child_value(2), my_array.child_value(2));
    }

    #[test]
    fn gvariant_variable_array_long() {
        let mut long_string = String::new();
        for _ in 0..244 {
            long_string.push('a');
        }

        let strings: &[&str; 3] = &["abc", "test", &long_string];

        let ref_slice = strings.map(|b| glib::ToVariant::to_variant(&b));
        let ref_array =
            glib::Variant::array_from_iter_with_type(glib::VariantTy::STRING, ref_slice)
                .normal_form();
        let ref_data = ref_array.data();

        let my_slice = strings.map(|b| super::ToVariant::to_variant(&b));
        let my_array = Variant::array_from_iter_with_type(VariantTy::STRING, my_slice);
        let my_data = my_array.data();
        assert_bytes_eq(ref_data, &my_data, "test");
        assert_eq!(my_array.n_children(), 3);

        assert_eq!(ref_array.child_value(0), my_array.child_value(0));
        assert_eq!(ref_array.child_value(1), my_array.child_value(1));
        assert_eq!(ref_array.child_value(2), my_array.child_value(2));
    }

    #[test]
    fn gvariant_tuple() {
        let num1: u8 = 8;
        let num2: u32 = 42;
        let bytes1: &[u8; 5] = &[1, 2, 3, 4, 5];
        let bytes2: &[u8; 5] = &[10, 9, 8, 7, 6];
        let num3: u8 = 80;
        let num4: u32 = 4200;
        let bytes3: &[u8; 5] = &[100, 90, 80, 70, 60];

        let ref_num1 = glib::ToVariant::to_variant(&num1).normal_form();
        let ref_num2 = glib::ToVariant::to_variant(&num2).normal_form();
        let ref_bytes1 = glib::ToVariant::to_variant(bytes1.as_slice()).normal_form();
        let ref_bytes2 = glib::ToVariant::to_variant(bytes2.as_slice()).normal_form();
        let ref_num3 = glib::ToVariant::to_variant(&num3).normal_form();
        let ref_num4 = glib::ToVariant::to_variant(&num4).normal_form();
        let ref_bytes3 = glib::ToVariant::to_variant(bytes3.as_slice()).normal_form();
        let ref_tuple = glib::Variant::tuple_from_iter(&[
            ref_num1, ref_num2, ref_bytes1, ref_bytes2, ref_num3, ref_num4, ref_bytes3,
        ]);
        let ref_data = ref_tuple.data();

        let my_num1 = super::ToVariant::to_variant(&num1);
        let my_num2 = super::ToVariant::to_variant(&num2);
        let my_bytes1 = Variant::from_data_with_type(bytes1.to_vec(), VariantTy::BYTE_STRING);
        let my_bytes2 = Variant::from_data_with_type(bytes2.to_vec(), VariantTy::BYTE_STRING);
        let my_num3 = super::ToVariant::to_variant(&num3);
        let my_num4 = super::ToVariant::to_variant(&num4);
        let my_bytes3 = Variant::from_data_with_type(bytes3.to_vec(), VariantTy::BYTE_STRING);
        let my_tuple = Variant::tuple_from_iter(&[
            my_num1, my_num2, my_bytes1, my_bytes2, my_num3, my_num4, my_bytes3,
        ]);
        let my_data = my_tuple.data();
        assert_bytes_eq(ref_data, &my_data, "test");

        assert_eq!(ref_tuple.child_value(0), my_tuple.child_value(0));
        assert_eq!(ref_tuple.child_value(1), my_tuple.child_value(1));
        assert_eq!(ref_tuple.child_value(2), my_tuple.child_value(2));
        assert_eq!(ref_tuple.child_value(3), my_tuple.child_value(3));
        assert_eq!(ref_tuple.child_value(4), my_tuple.child_value(4));
        assert_eq!(ref_tuple.child_value(5), my_tuple.child_value(5));
        assert_eq!(ref_tuple.child_value(6), my_tuple.child_value(6));
    }

    #[test]
    fn gvariant_nested_tuple() {
        let num1: u8 = 8;
        let num2: u32 = 42;
        let bytes: &[u8; 5] = &[1, 2, 3, 4, 5];

        let ref_num1 = glib::ToVariant::to_variant(&num1).normal_form();
        let ref_num2 = glib::ToVariant::to_variant(&num2).normal_form();
        let ref_bytes = glib::ToVariant::to_variant(bytes.as_slice()).normal_form();
        let ref_tuple = glib::Variant::tuple_from_iter(&[ref_num1, ref_num2, ref_bytes]);
        let ref_data = ref_tuple.data();

        let my_num1 = super::ToVariant::to_variant(&num1);
        let my_num2 = super::ToVariant::to_variant(&num2);
        let my_bytes = Variant::from_data_with_type(bytes.to_vec(), VariantTy::BYTE_STRING);
        let my_tuple = Variant::tuple_from_iter(&[my_num1, my_num2, my_bytes]);
        let my_data = my_tuple.data();
        assert_bytes_eq(ref_data, &my_data, "test");
    }

    #[test]
    fn gvariant_variant() {
        let num: u32 = 42;
        let bytes: &[u8; 5] = &[1, 2, 3, 4, 5];

        let ref_num = glib::ToVariant::to_variant(&num).normal_form();
        let ref_bytes = glib::ToVariant::to_variant(bytes.as_slice()).normal_form();
        let ref_tuple = glib::Variant::tuple_from_iter(&[ref_num, ref_bytes]);
        let ref_variant = glib::Variant::from_variant(&ref_tuple);
        let ref_data = ref_variant.data();

        let my_num = super::ToVariant::to_variant(&num);
        let my_bytes = Variant::from_data_with_type(bytes.to_vec(), VariantTy::BYTE_STRING);
        let my_tuple = Variant::tuple_from_iter(&[my_num, my_bytes]);
        let my_variant = Variant::from_variant(&my_tuple);
        let my_data = my_variant.data();

        assert_bytes_eq(ref_data, &my_data, "test");
    }
}
