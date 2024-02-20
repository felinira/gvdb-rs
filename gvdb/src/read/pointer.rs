#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Pointer {
    start: u32,
    end: u32,
}

impl Pointer {
    #[allow(unused)]
    pub(crate) const NULL: Self = Self { start: 0, end: 0 };

    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: (start as u32).to_le(),
            end: (end as u32).to_le(),
        }
    }

    pub fn start(&self) -> u32 {
        u32::from_le(self.start)
    }

    pub fn end(&self) -> u32 {
        u32::from_le(self.end)
    }

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
        let pointer2 = pointer.clone();
        println!("{:?}", pointer2);
    }

    #[test]
    fn no_panic_invalid_size() {
        let invalid_ptr = Pointer::new(100, 0);
        let size = invalid_ptr.size();
        assert_eq!(size, 0);
    }
}
