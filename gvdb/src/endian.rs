#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The representation of endianness used for this crate
pub enum Endian {
    /// Values encoded in little endian representation
    Little,
    /// Values encoded in big endian representation
    Big,
}

impl Endian {
    /// The native endianness of the target platform
    #[cfg(feature = "glib")]
    pub(crate) fn native() -> Self {
        if cfg!(target_endian = "little") {
            Self::Little
        } else {
            Self::Big
        }
    }
}

impl From<Endian> for zvariant::Endian {
    fn from(value: Endian) -> Self {
        match value {
            Endian::Little => zvariant::Endian::Little,
            Endian::Big => zvariant::Endian::Big,
        }
    }
}
