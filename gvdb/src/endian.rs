#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The representation of endianness used for this crate
pub enum Endian {
    /// Values encoded in little endian representation
    Little,
    /// Values encoded in big endian representation
    Big,
}

#[cfg(feature = "zvariant")]
impl From<Endian> for zvariant::Endian {
    fn from(value: Endian) -> Self {
        match value {
            Endian::Little => zvariant::Endian::Little,
            Endian::Big => zvariant::Endian::Big,
        }
    }
}

impl Endian {
    pub const NATIVE: Self = if cfg!(target_endian = "little") {
        Self::Little
    } else {
        Self::Big
    };

    pub fn is_native(&self) -> bool {
        self == &Self::NATIVE
    }

    pub fn is_byteswap(&self) -> bool {
        self != &Self::NATIVE
    }
}

impl std::ops::Not for Endian {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Endian::Little => Endian::Big,
            Endian::Big => Endian::Little,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Endian;

    #[test]
    fn is() {
        let endian = Endian::NATIVE;
        assert!(endian.is_native());
        assert!((!endian).is_byteswap());
        assert!((!!endian).is_native());
    }

    #[test]
    fn not() {
        let endian = Endian::Little;
        assert_eq!(endian, endian);
        assert_eq!(!endian, Endian::Big);
        assert_eq!(!!endian, endian);
        assert_eq!(!Endian::Big, Endian::Little);
    }
}
