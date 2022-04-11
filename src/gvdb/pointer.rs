#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GvdbPointer {
    start: u32,
    end: u32,
}

impl GvdbPointer {
    pub const NULL: Self = Self { start: 0, end: 0 };

    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    pub fn swap_bytes(&self) -> Self {
        Self {
            start: self.start.swap_bytes(),
            end: self.end.swap_bytes(),
        }
    }

    pub fn start(&self) -> u32 {
        u32::from_le(self.start)
    }

    pub fn end(&self) -> u32 {
        u32::from_le(self.end)
    }

    pub fn size(&self) -> u32 {
        self.end() - self.start()
    }
}
