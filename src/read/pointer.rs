#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GvdbPointer {
    start: u32,
    end: u32,
}

impl GvdbPointer {
    pub const NULL: Self = Self { start: 0, end: 0 };

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
        (self.end() - self.start()) as usize
    }
}
