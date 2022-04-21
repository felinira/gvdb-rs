use std::borrow::{Borrow, Cow};
use std::cmp::max;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BoolError {
    pub message: Cow<'static, str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TypeInfo {
    pub alignment: u8,
    pub fixed_size: usize,
    pub element_fixed_size: usize,
    pub n_members: usize,
    pub tuple_type_info: Option<TupleTypeInfo>,
}

impl TypeInfo {
    pub fn variant(alignment: u8, fixed_size: usize) -> Self {
        Self {
            alignment,
            fixed_size,
            element_fixed_size: fixed_size,
            n_members: 1,
            tuple_type_info: None,
        }
    }

    pub fn maybe(alignment: u8, fixed_size: usize) -> Self {
        Self {
            alignment,
            fixed_size,
            element_fixed_size: 0,
            n_members: 1,
            tuple_type_info: None,
        }
    }

    pub fn array(alignment: u8, fixed_size: usize, element_fixed_size: usize) -> Self {
        Self {
            alignment,
            fixed_size,
            element_fixed_size,
            n_members: 0,
            tuple_type_info: None,
        }
    }

    pub fn tuple(
        alignment: u8,
        fixed_size: usize,
        n_members: usize,
        tuple_type_info: TupleTypeInfo,
    ) -> Self {
        Self {
            alignment,
            fixed_size,
            element_fixed_size: 0,
            n_members,
            tuple_type_info: Some(tuple_type_info),
        }
    }

    pub fn fixed_aligned(size: usize) -> Self {
        Self {
            alignment: size as u8,
            fixed_size: size,
            element_fixed_size: 0,
            n_members: 0,
            tuple_type_info: None,
        }
    }

    pub fn unaligned() -> Self {
        Self {
            alignment: 1,
            fixed_size: 0,
            element_fixed_size: 0,
            n_members: 0,
            tuple_type_info: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TupleTypeInfo {
    pub member_types: Vec<VariantType>,
    pub member_type_info: Vec<TypeInfo>,
    pub n_unsized_members: usize,
}

/// The type of a [`Variant`][super::Variant]
///
/// Internally this is just a string that meets certain invariants
#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct VariantTy(pub [u8]);

impl VariantTy {
    // basic type characters
    /// The type character for a bool
    pub const CLASS_BOOLEAN: u8 = b'b';

    /// The type character for a byte
    pub const CLASS_BYTE: u8 = b'y';

    /// The type character for an i16
    pub const CLASS_INT16: u8 = b'n';

    /// The type character for an u16
    pub const CLASS_UINT16: u8 = b'q';

    /// The type character for an i32
    pub const CLASS_INT32: u8 = b'i';

    /// The type character for am i32
    pub const CLASS_UINT32: u8 = b'u';

    /// The type character for an i64
    pub const CLASS_INT64: u8 = b'x';

    /// The type character for an u64
    pub const CLASS_UINT64: u8 = b't';

    /// The type character for an f64
    pub const CLASS_DOUBLE: u8 = b'd';

    /// The type character for a string
    pub const CLASS_STRING: u8 = b's';

    /// The type character for a variant container
    pub const CLASS_VARIANT: u8 = b'v';

    /// The type character for a maybe container
    pub const CLASS_MAYBE: u8 = b'm';

    /// The type character for an array
    pub const CLASS_ARRAY: u8 = b'a';

    /// The type character for a tuple
    pub const CLASS_TUPLE: u8 = b'(';

    /// The type character for a dict entry
    pub const CLASS_DICT_ENTRY: u8 = b'{';

    /// The type character for any tuple type
    pub const CLASS_ANY_TUPLE: u8 = b'r';

    /// The type character for any basic type
    pub const CLASS_ANY_BASIC: u8 = b'?';

    /// The type character for any type
    pub const CLASS_ANY: u8 = b'*';

    // type strings
    /// This type directly converts to bool
    pub const BOOLEAN: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_BOOLEAN]) };

    /// This type directly converts to [`u8`]
    pub const BYTE: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_BYTE]) };

    /// This type directly converts to [`i16`]
    pub const INT16: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_INT16]) };

    /// This type directly converts to [`u16`]
    pub const UINT16: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_UINT16]) };

    /// This type directly converts to [`i32`]
    pub const INT32: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_INT32]) };

    /// This type directly converts to [`u32`]
    pub const UINT32: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_UINT32]) };

    /// This type directly converts to [`i64`]
    pub const INT64: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_INT64]) };

    /// This type directly converts to [`u64`]
    pub const UINT64: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_UINT64]) };

    /// This type directly converts to [`f64`]
    pub const DOUBLE: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_DOUBLE]) };

    /// This type directly converts to [`String`]
    pub const STRING: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_STRING]) };

    /// A container with associated type information
    pub const VARIANT: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_VARIANT]) };

    /// An empty tuple
    pub const UNIT: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"()") };

    // Supertypes
    /// Any type
    pub const ANY: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(&[b'*']) };

    /// Any basic type
    pub const BASIC: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(&[b'?']) };

    /// Any tuple type
    pub const TUPLE: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"r") };

    /// Any dict entry
    pub const DICT_ENTRY: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"{?*}") };

    /// Any dictionary
    pub const DICTIONARY: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"a{?*}") };

    /// Any maybe type
    pub const MAYBE: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_MAYBE, b'*']) };

    /// Any array
    pub const ARRAY: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(&[Self::CLASS_ARRAY, b'*']) };

    // Compound types
    /// An array of strings
    pub const STRING_ARRAY: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"as") };

    /// An array of bytes
    pub const BYTE_STRING: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"ay") };

    /// An array of byte arrays
    pub const BYTE_STRING_ARRAY: &'static VariantTy =
        unsafe { VariantTy::from_slice_unchecked(b"ayy") };

    /// A dictionary of variant types
    pub const VARDICT: &'static VariantTy = unsafe { VariantTy::from_slice_unchecked(b"a{sv}") };

    /// Create a VariantTy without checking if the type string is valid
    ///
    /// # Safety
    ///
    /// This may not actually cause undefined behavior. The only reason this function is marked
    /// unsafe is because it is possible to invalidate the type string invariants with this
    /// function.
    pub const unsafe fn from_slice_unchecked(type_str: &[u8]) -> &VariantTy {
        std::mem::transmute::<&[u8], &VariantTy>(type_str)
    }

    /// Return the bytes that compose this type string
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Return the type string
    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    /// Create a new type string. This will fail if the type string is not valid
    pub fn new(type_string: &str) -> Result<&Self, BoolError> {
        if Self::type_string_is_valid(type_string) {
            let bytes = type_string.as_bytes();
            Ok(unsafe { Self::from_slice_unchecked(bytes) })
        } else {
            Err(BoolError {
                message: Cow::Owned(format!("Invalid type string: '{}'", type_string)),
            })
        }
    }

    /// Strips any extra bytes from self
    pub fn normalized(&self) -> &Self {
        unsafe { Self::from_slice_unchecked(&self.0[0..self.string_length()]) }
    }

    fn is_basic_char(char: u8) -> bool {
        matches!(
            char,
            Self::CLASS_BOOLEAN
                | Self::CLASS_BYTE
                | Self::CLASS_INT16
                | Self::CLASS_UINT16
                | Self::CLASS_INT32
                | Self::CLASS_UINT32
                | Self::CLASS_INT64
                | Self::CLASS_UINT64
                | Self::CLASS_DOUBLE
                | Self::CLASS_STRING
                | Self::CLASS_ANY_BASIC
        )
    }

    /// Return whether this type string corresponds to a basic / non-container type (like int or string)
    pub fn is_basic(&self) -> bool {
        self.0.len() == 1 && Self::is_basic_char(self.0[0])
    }

    /// Return whether this type is a container
    pub fn is_container(&self) -> bool {
        matches!(
            self.0[0],
            Self::CLASS_ARRAY
                | Self::CLASS_MAYBE
                | Self::CLASS_TUPLE
                | Self::CLASS_ANY_TUPLE
                | Self::CLASS_DICT_ENTRY
                | Self::CLASS_VARIANT
        )
    }

    /// Return whether this type is a maybe (Option) type
    pub fn is_maybe(&self) -> bool {
        self.0[0] == Self::CLASS_MAYBE
    }

    /// Return whether this type is an array
    pub fn is_array(&self) -> bool {
        self.0[0] == Self::CLASS_ARRAY
    }

    /// Return whether this type is a tuple
    pub fn is_tuple(&self) -> bool {
        matches!(self.0[0], Self::CLASS_TUPLE | Self::CLASS_ANY_TUPLE)
    }

    /// Return whether this type is a dict entry
    pub fn is_dict_entry(&self) -> bool {
        self.0[0] == Self::CLASS_DICT_ENTRY
    }

    /// Return whether this type is of type "variant"
    pub fn is_variant(&self) -> bool {
        self.0[0] == Self::CLASS_VARIANT
    }

    fn string_length_inner(type_str: &[u8]) -> usize {
        let mut brackets = 0;
        let mut index = 0;
        loop {
            let mut chr = type_str[index];
            while chr == Self::CLASS_ARRAY || chr == Self::CLASS_MAYBE {
                index += 1;
                chr = type_str[index];
            }

            if chr == Self::CLASS_TUPLE || chr == Self::CLASS_MAYBE {
                brackets += 1;
            } else if chr == b')' || chr == b'}' {
                brackets -= 1;
            }

            index += 1;

            if brackets == 0 {
                break;
            }
        }

        index
    }

    /// Return the actual length of the type string without any extra chars at the end
    pub fn string_length(&self) -> usize {
        Self::string_length_inner(&self.0)
    }

    /// Return whether this type is a subtype of the specified supertype
    pub fn is_subtype_of(&self, supertype: &VariantTy) -> bool {
        let mut type_string = &self.0;
        for supertype_char in &supertype.0 {
            if type_string[0] == *supertype_char {
                type_string = &type_string[1..];
            } else if type_string[0] == b')' {
                return false;
            } else {
                let target_type = unsafe { Self::from_slice_unchecked(type_string) };
                match *supertype_char {
                    Self::CLASS_ANY_TUPLE => {
                        if !target_type.is_tuple() {
                            return false;
                        }
                    }
                    Self::CLASS_ANY_BASIC => {
                        if !target_type.is_basic() {
                            return false;
                        }
                    }
                    Self::CLASS_ANY => {}
                    _ => return false,
                }

                type_string = &type_string[target_type.string_length()..];
            }
        }

        true
    }

    fn type_string_is_valid_inner(type_str: &mut &[u8], depth_limit: usize) -> Option<()> {
        if depth_limit == 0 || type_str.is_empty() {
            return None;
        }

        let char = *type_str.get(0)?;

        // Advance the pointer for next invocation
        *type_str = &type_str[1..];

        if Self::is_basic_char(char)
            || matches!(
                char,
                Self::CLASS_VARIANT
                    | Self::CLASS_ANY_TUPLE
                    | Self::CLASS_ANY_BASIC
                    | Self::CLASS_ANY
            )
        {
            Some(())
        } else if char == Self::CLASS_TUPLE {
            while *type_str.get(0)? != b')' {
                Self::type_string_is_valid_inner(type_str, depth_limit - 1)?;
            }

            *type_str = &type_str[1..];
            Some(())
        } else if char == Self::CLASS_DICT_ENTRY {
            if !Self::is_basic_char(*type_str.get(0)?) {
                return None;
            }

            *type_str = &type_str[1..];
            Self::type_string_is_valid_inner(type_str, depth_limit - 1)?;

            if *type_str.get(0)? != b'}' {
                return None;
            }

            *type_str = &type_str[1..];
            Some(())
        } else if char == Self::CLASS_MAYBE || char == Self::CLASS_ARRAY {
            Self::type_string_is_valid_inner(type_str, depth_limit - 1)
        } else {
            None
        }
    }

    /// Is this type string valid?
    pub fn type_string_is_valid(type_str: &str) -> bool {
        let mut type_str = type_str.as_bytes();
        if !type_str.is_empty() {
            Self::type_string_is_valid_inner(&mut type_str, 128).is_some() && type_str.is_empty()
        } else {
            false
        }
    }

    fn type_info_inner(&self) -> (TypeInfo, usize) {
        let first_char = self.0[0];

        match first_char {
            Self::CLASS_BOOLEAN => (TypeInfo::fixed_aligned(1), 1),
            Self::CLASS_BYTE => (TypeInfo::fixed_aligned(1), 1),
            Self::CLASS_INT16 => (TypeInfo::fixed_aligned(2), 1),
            Self::CLASS_UINT16 => (TypeInfo::fixed_aligned(2), 1),
            Self::CLASS_INT32 => (TypeInfo::fixed_aligned(4), 1),
            Self::CLASS_UINT32 => (TypeInfo::fixed_aligned(4), 1),
            Self::CLASS_INT64 => (TypeInfo::fixed_aligned(8), 1),
            Self::CLASS_UINT64 => (TypeInfo::fixed_aligned(8), 1),
            Self::CLASS_DOUBLE => (TypeInfo::fixed_aligned(8), 1),
            Self::CLASS_STRING => (TypeInfo::unaligned(), 1),
            Self::CLASS_VARIANT => (TypeInfo::variant(8, 0), 1),
            Self::CLASS_MAYBE => {
                let element_type = self.type_element();
                let (element_typeinfo, consumed) = element_type.type_info_inner();
                (
                    TypeInfo::maybe(element_typeinfo.alignment, element_typeinfo.fixed_size),
                    consumed + 1,
                )
            }
            Self::CLASS_TUPLE => {
                if self.0.is_empty() {
                    (TypeInfo::fixed_aligned(1), 1)
                } else {
                    let mut tuple_type_info = TupleTypeInfo {
                        member_types: vec![],
                        member_type_info: vec![],
                        n_unsized_members: 0,
                    };

                    let mut tuple_alignment = 0;
                    let mut size: usize = 0;
                    let mut offset = 1;
                    let mut n_members = 0;

                    while offset < self.0.len() {
                        let rest = &self.0[offset..self.0.len()];
                        if rest.starts_with(b")") {
                            break;
                        }

                        let member_type = unsafe { VariantTy::from_slice_unchecked(rest) };
                        tuple_type_info
                            .member_types
                            .push(member_type.normalized().to_owned());
                        let (type_info, consumed) = member_type.type_info_inner();

                        offset += consumed;
                        tuple_alignment = max(tuple_alignment, type_info.alignment);
                        let alignment = type_info.alignment as usize;

                        if type_info.fixed_size == 0 {
                            tuple_type_info.n_unsized_members += 1;
                        }

                        if size % alignment as usize != 0 {
                            size += alignment - size % alignment as usize;
                        }

                        size += type_info.fixed_size;
                        tuple_type_info.member_type_info.push(type_info);

                        n_members += 1;
                    }

                    if tuple_type_info.n_unsized_members > 0 {
                        size = 0;
                    }

                    (
                        TypeInfo::tuple(tuple_alignment, size, n_members, tuple_type_info),
                        offset,
                    )
                }
            }
            Self::CLASS_ARRAY => unsafe {
                // We don't know anything about the size, but our alignment is the alignment of the data type
                let (type_info, consumed) =
                    VariantTy::from_slice_unchecked(&[self.0[1]]).type_info_inner();
                (
                    TypeInfo::array(type_info.alignment, 0, type_info.fixed_size),
                    consumed + 1,
                )
            },
            _ => (TypeInfo::unaligned(), 1),
        }
    }

    /// Return the type information for this type and any contained sub types
    pub(super) fn type_info(&self) -> TypeInfo {
        self.type_info_inner().0
    }

    /// Return the element type of this variant type.
    ///
    /// # Panics
    ///
    /// This function panics if not called with an array or maybe type.
    pub fn type_element(&self) -> &VariantTy {
        assert!(self.0[0] == b'a' || self.0[0] == b'm');
        unsafe { Self::from_slice_unchecked(&self.0[1..]) }
    }

    /// Iterate over the types of this variant type.
    ///
    /// # Panics
    ///
    /// This function panics if not called with a tuple or dictionary entry type.
    pub fn tuple_types(&self) -> VariantTyIterator {
        VariantTyIterator::new(self).expect("VariantTy does not represent a tuple")
    }

    /// Return the first type of this variant type.
    ///
    /// # Panics
    ///
    /// This function panics if not called with a tuple or dictionary entry type.
    pub fn first(&self) -> Option<&VariantTy> {
        assert!(self.0[0] == b'(' || self.0[0] == b'{');
        if self.0[1] == b')' {
            None
        } else {
            Some(unsafe { Self::from_slice_unchecked(&self.0[1..]) })
        }
    }

    /// Return the next type of this variant type.
    pub fn next(&self) -> Option<&VariantTy> {
        let this_len = self.string_length();
        let next_type = unsafe { Self::from_slice_unchecked(&self.0[this_len..]) };
        if next_type.0.is_empty() || next_type.0[0] == b')' || next_type.0[0] == b'}' {
            None
        } else {
            Some(next_type)
        }
    }

    /// Return the number of items in this variant type.
    pub fn n_items(&self) -> usize {
        let mut count = 0;
        let mut this_type = self.first();

        while let Some(this) = this_type {
            count += 1;
            this_type = this.next()
        }

        count
    }

    /// Return the key type of this variant type.
    ///
    /// # Panics
    ///
    /// This function panics if not called with a dictionary entry type.
    pub fn key(&self) -> &Self {
        assert_eq!(self.0[0], b'{');
        unsafe { Self::from_slice_unchecked(&self.0[1..]) }
    }

    /// Return the value type of this variant type.
    ///
    /// # Panics
    ///
    /// This function panics if not called with a dictionary entry type.
    pub fn value(&self) -> &Self {
        assert_eq!(self.0[0], b'{');
        self.key().next().unwrap()
    }
}

impl Debug for VariantTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_basic() {
            match self.0[0] {
                VariantTy::CLASS_BOOLEAN => write!(f, "bool"),
                VariantTy::CLASS_BYTE => write!(f, "byte"),
                VariantTy::CLASS_INT16 => write!(f, "int16"),
                VariantTy::CLASS_UINT16 => write!(f, "uin16"),
                VariantTy::CLASS_INT32 => write!(f, "int32"),
                VariantTy::CLASS_UINT32 => write!(f, "uint32"),
                VariantTy::CLASS_INT64 => write!(f, "int64"),
                VariantTy::CLASS_UINT64 => write!(f, "uint64"),
                VariantTy::CLASS_DOUBLE => write!(f, "double"),
                VariantTy::CLASS_STRING => write!(f, "string"),
                _ => write!(f, "?"),
            }
        } else if self.is_array() {
            write!(f, "{:?}[]", self.type_element())
        } else if self.is_tuple() {
            write!(f, "(")?;

            for (index, elem) in self.tuple_types().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }

                write!(f, "{:?}", elem)?;
            }

            write!(f, ")")
        } else if self.is_maybe() {
            write!(f, "{:?}?", self.type_element())
        } else if self.is_variant() {
            write!(f, "Variant")
        } else {
            Debug::fmt(&self, f)
        }
    }
}

impl Display for VariantTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ToOwned for VariantTy {
    type Owned = VariantType;

    fn to_owned(&self) -> Self::Owned {
        let owned = self.0.to_vec().into_boxed_slice();

        VariantType {
            inner: unsafe { std::mem::transmute(owned) },
        }
    }
}

#[cfg(feature = "glib")]
impl PartialEq<glib::VariantTy> for VariantTy {
    fn eq(&self, other: &glib::VariantTy) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(feature = "glib")]
impl PartialEq<VariantTy> for glib::VariantTy {
    fn eq(&self, other: &VariantTy) -> bool {
        other.eq(self)
    }
}

/// An Iterator over the child elements of a Variant Type
pub struct VariantTyIterator<'a> {
    elem: Option<&'a VariantTy>,
}

impl<'a> VariantTyIterator<'a> {
    // rustdoc-stripper-ignore-next
    /// Creates a new iterator over the types of the specified [VariantTy].
    ///
    /// Returns `Ok` if the type is a definite tuple or dictionary entry type,
    /// `Err` otherwise.
    pub fn new(ty: &'a VariantTy) -> Result<Self, BoolError> {
        if (ty.is_tuple() && ty != VariantTy::TUPLE) || ty.is_dict_entry() {
            Ok(Self { elem: ty.first() })
        } else {
            Err(BoolError {
                message: Cow::Borrowed("Expected a definite tuple or dictionary entry type"),
            })
        }
    }
}

impl<'a> Iterator for VariantTyIterator<'a> {
    type Item = &'a VariantTy;

    #[doc(alias = "g_variant_type_next")]
    fn next(&mut self) -> Option<Self::Item> {
        let elem = self.elem?;
        self.elem = elem.next();
        Some(elem.normalized())
    }
}

/// An owned version of [`VariantTy`]
#[repr(transparent)]
#[derive(PartialEq)]
pub struct VariantType {
    inner: Box<VariantTy>,
}

impl VariantType {
    /// Create a new VariantType from a boxed VariantTy
    ///
    /// ## `boxed`
    /// a boxed [`VariantTy`]
    ///
    /// # Returns
    ///
    /// a [`VariantType`]
    pub const fn from_boxed(boxed: Box<VariantTy>) -> Self {
        unsafe { std::mem::transmute(boxed) }
    }

    /// Create a new VariantType by creating a copy of a type string.
    ///
    /// ## `type_string`
    /// a valid type string
    ///
    /// Returns `Ok` if the string is a valid type string, `Err` otherwise.
    pub fn new(type_string: &str) -> Result<Self, BoolError> {
        VariantTy::new(type_string).map(ToOwned::to_owned)
    }

    /// Creates a `VariantType` from an array element type.
    /// Constructs the type corresponding to an array of elements of the
    /// type `type_`.
    ///
    /// It is appropriate to call `g_variant_type_free()` on the return value.
    /// ## `elem_type`
    /// a [`VariantType`][crate::no_glib::VariantType]
    ///
    /// # Returns
    ///
    /// a new array [`VariantType`][crate::no_glib::VariantType]
    pub fn new_array(elem_type: &VariantTy) -> Result<Self, BoolError> {
        let mut type_str = String::new();
        type_str.push('a');
        type_str.push_str(unsafe { std::str::from_utf8_unchecked(&elem_type.0) });
        Self::new(&type_str)
    }

    /// Creates a `VariantType` from a maybe element type.
    /// Constructs the type corresponding to a maybe instance containing
    /// type `type_` or Nothing.
    ///
    /// ## `element`
    /// a [`VariantType`][crate::no_glib::VariantType]
    ///
    /// # Returns
    ///
    /// a new maybe [`VariantType`][crate::no_glib::VariantType]
    #[doc(alias = "g_variant_type_new_maybe")]
    pub fn new_maybe(child_type: &VariantTy) -> Result<Self, BoolError> {
        let mut type_str = String::new();
        type_str.push('m');
        type_str.push_str(unsafe { std::str::from_utf8_unchecked(&child_type.0) });
        Self::new(&type_str)
    }

    /// Constructs a new tuple type, from `items`.
    ///
    /// ## `items`
    /// a slice of `&VariantTy`, one for each item
    ///
    /// # Returns
    ///
    /// a new tuple [`VariantType`][crate::no_glib::VariantType]
    #[doc(alias = "g_variant_type_new_tuple")]
    pub fn new_tuple<T: AsRef<VariantTy>, I: IntoIterator<Item = T>>(items: I) -> VariantType {
        let mut type_str = String::from('(');

        for ty in items {
            type_str.push_str(ty.as_ref().as_str());
        }

        type_str.push(')');

        VariantType::from_string(type_str).unwrap()
    }

    /// Tries to create a `VariantType` from an owned string.
    ///
    /// Returns `Ok` if the string is a valid type string, `Err` otherwise.
    pub fn from_string(type_string: impl Into<String>) -> Result<VariantType, BoolError> {
        Self::new(&type_string.into())
    }
}

impl Clone for VariantType {
    fn clone(&self) -> Self {
        Self::new(self.inner.as_str()).unwrap()
    }
}

impl Debug for VariantType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl Display for VariantType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl AsRef<VariantTy> for VariantType {
    fn as_ref(&self) -> &VariantTy {
        self
    }
}

impl Borrow<VariantTy> for VariantType {
    fn borrow(&self) -> &VariantTy {
        self
    }
}

impl Deref for VariantType {
    type Target = VariantTy;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Box<VariantTy>> for VariantType {
    fn from(boxed: Box<VariantTy>) -> Self {
        Self::from_boxed(boxed)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn type_string_is_valid() {
        // basic types
        assert!(VariantTy::type_string_is_valid("b"));
        assert!(VariantTy::type_string_is_valid("y"));
        assert!(VariantTy::type_string_is_valid("n"));
        assert!(VariantTy::type_string_is_valid("q"));
        assert!(VariantTy::type_string_is_valid("i"));
        assert!(VariantTy::type_string_is_valid("u"));
        assert!(VariantTy::type_string_is_valid("x"));
        assert!(VariantTy::type_string_is_valid("t"));
        assert!(VariantTy::type_string_is_valid("d"));
        assert!(VariantTy::type_string_is_valid("s"));
        assert!(VariantTy::type_string_is_valid("v"));
        assert!(VariantTy::type_string_is_valid("r"));
        assert!(VariantTy::type_string_is_valid("?"));
        assert!(VariantTy::type_string_is_valid("*"));
        assert!(VariantTy::type_string_is_valid("v"));

        assert!(!VariantTy::type_string_is_valid("z"));
        assert!(!VariantTy::type_string_is_valid("c"));
        assert!(!VariantTy::type_string_is_valid("by"));
        assert!(!VariantTy::type_string_is_valid("dsa"));

        // maybe
        assert!(VariantTy::type_string_is_valid("mb"));
        assert!(VariantTy::type_string_is_valid("my"));
        assert!(VariantTy::type_string_is_valid("mn"));
        assert!(VariantTy::type_string_is_valid("ms"));
        assert!(VariantTy::type_string_is_valid("mv"));

        assert!(!VariantTy::type_string_is_valid("msq"));
        assert!(!VariantTy::type_string_is_valid("mva"));

        // array
        assert!(VariantTy::type_string_is_valid("ay"));
        assert!(VariantTy::type_string_is_valid("au"));
        assert!(VariantTy::type_string_is_valid("at"));
        assert!(VariantTy::type_string_is_valid("a?"));
        assert!(VariantTy::type_string_is_valid("a*"));

        assert!(!VariantTy::type_string_is_valid("abc"));
        assert!(!VariantTy::type_string_is_valid("aqq"));
        assert!(!VariantTy::type_string_is_valid("a"));
        assert!(!VariantTy::type_string_is_valid("auu"));

        // tuple
        assert!(VariantTy::type_string_is_valid("()"));
        assert!(VariantTy::type_string_is_valid("(uus)"));
        assert!(VariantTy::type_string_is_valid("(uy)"));

        assert!(!VariantTy::type_string_is_valid("(uus"));
        assert!(!VariantTy::type_string_is_valid("(uus))"));
        assert!(!VariantTy::type_string_is_valid("((uus)"));

        // dict
        assert!(VariantTy::type_string_is_valid("{uu}"));
        assert!(VariantTy::type_string_is_valid("{ui}"));
        assert!(VariantTy::type_string_is_valid("a{us}"));
        assert!(VariantTy::type_string_is_valid("a{su}"));
        assert!(VariantTy::type_string_is_valid("a{u(us)}"));
        assert!(VariantTy::type_string_is_valid("a{uas}"));

        assert!(!VariantTy::type_string_is_valid("a{u}"));
        assert!(!VariantTy::type_string_is_valid("a{}"));
        assert!(!VariantTy::type_string_is_valid("a{"));
        assert!(!VariantTy::type_string_is_valid("a}"));
        assert!(!VariantTy::type_string_is_valid("a{uuu}"));

        // complex
        assert!(VariantTy::type_string_is_valid("(u(u(yu)))"));
        assert!(VariantTy::type_string_is_valid("(aaay(uay))"));
        assert!(VariantTy::type_string_is_valid("(uusa(uu))"));
        assert!(VariantTy::type_string_is_valid("(a{u(usa{us})}su)"));

        assert!(!VariantTy::type_string_is_valid("(uusa(uu)()"));
        assert!(!VariantTy::type_string_is_valid("(a{u(usa{us})su)"));
        assert!(!VariantTy::type_string_is_valid("(u(u(yu)))u"));
    }

    #[test]
    fn is_subtype_of() {
        assert!(VariantTy::new("ay")
            .unwrap()
            .is_subtype_of(&VariantTy::new("a?").unwrap()));
        assert!(VariantTy::new("(uu)")
            .unwrap()
            .is_subtype_of(&VariantTy::new("r").unwrap()));
        assert!(VariantTy::new("(a{u(usa{us})}su)")
            .unwrap()
            .is_subtype_of(&VariantTy::new("*").unwrap()));
        assert!(VariantTy::new("as")
            .unwrap()
            .is_subtype_of(&VariantTy::new("a*").unwrap()));
        assert!(VariantTy::new("s")
            .unwrap()
            .is_subtype_of(&VariantTy::new("?").unwrap()));

        assert!(!VariantTy::new("ay")
            .unwrap()
            .is_subtype_of(&VariantTy::new("?").unwrap()));
        assert!(!VariantTy::new("()")
            .unwrap()
            .is_subtype_of(&VariantTy::new("?").unwrap()));
        assert!(!VariantTy::new("mu")
            .unwrap()
            .is_subtype_of(&VariantTy::new("?").unwrap()));
        assert!(!VariantTy::new("{uu}")
            .unwrap()
            .is_subtype_of(&VariantTy::new("?").unwrap()));
        assert!(!VariantTy::new("u")
            .unwrap()
            .is_subtype_of(&VariantTy::new("au").unwrap()));
        assert!(!VariantTy::new("(au)")
            .unwrap()
            .is_subtype_of(&VariantTy::new("(auu)").unwrap()));
    }

    #[test]
    fn string_len() {
        unsafe {
            assert_eq!(VariantTy::new("(ay)").unwrap().string_length(), 4);
            assert_eq!(VariantTy::from_slice_unchecked(b"(u)ay").string_length(), 3);
            assert_eq!(VariantTy::from_slice_unchecked(b"uu").string_length(), 1);
        }
    }
}
