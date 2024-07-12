/// A slice of little endian u32
#[derive(Debug, Clone, Copy)]
pub(crate) struct SliceLEu32<'a>(pub &'a [[u8; 4]]);

impl<'a> SliceLEu32<'a> {
    pub fn get(&self, index: usize) -> Option<u32> {
        self.0.get(index).map(|i| u32::from_le_bytes(*i))
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
