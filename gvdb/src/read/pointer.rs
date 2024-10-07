use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// A pointer internal to the GVDB file.
///
/// GVDB files use pointer structs with global start and end locations. Pointers
/// are *always* little-endian, independant of the file endianess.
///
/// It is possible to retrieve the bytes stored at this pointer by using
/// [`File::dereference()`](crate::read::File::dereference).
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Immutable, KnownLayout, FromBytes, IntoBytes)]
pub struct Pointer {
    start: u32,
    end: u32,
}

impl Pointer {
    #[allow(unused)]
    pub(crate) const NULL: Self = Self { start: 0, end: 0 };

    /// Create a new GVDB pointer. Pointers are always internally stored as little endian,
    /// so we convert the values here.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: (start as u32).to_le(),
            end: (end as u32).to_le(),
        }
    }

    /// Returns the start address of the pointer and convert them to target endianess.
    pub fn start(&self) -> u32 {
        u32::from_le(self.start)
    }

    /// Returns the end address of the pointer and convert them to target endianess.
    pub fn end(&self) -> u32 {
        u32::from_le(self.end)
    }

    /// Returns the number of bytes referenced by the pointer.
    pub fn size(&self) -> usize {
        self.end().saturating_sub(self.start()) as usize
    }
}

impl std::fmt::Debug for Pointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pointer")
            .field("start", &self.start())
            .field("end", &self.end())
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::read::Pointer;

    #[test]
    fn derives() {
        let pointer = Pointer::new(0, 2);
        let pointer2 = pointer;
        println!("{:?}", pointer2);
    }

    #[test]
    fn no_panic_invalid_size() {
        let invalid_ptr = Pointer::new(100, 0);
        let size = invalid_ptr.size();
        assert_eq!(size, 0);
    }
}
