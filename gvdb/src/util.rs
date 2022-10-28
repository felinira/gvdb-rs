/// Perform the djb2 hash function
pub fn djb_hash(key: &str) -> u32 {
    let mut hash_value: u32 = 5381;
    for char in key.bytes() {
        hash_value = hash_value.wrapping_mul(33).wrapping_add(char as u32);
    }

    hash_value
}

/// Align an arbitrary offset to a multiple of 2
/// The result is undefined for alignments that are not a multiple of 2
pub fn align_offset(offset: usize, alignment: usize) -> usize {
    //(alignment - (offset % alignment)) % alignment
    (offset + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod test {
    use super::align_offset;

    #[test]
    fn align() {
        assert_eq!(align_offset(17, 16), 32);

        assert_eq!(align_offset(13, 8), 16);

        assert_eq!(align_offset(1, 8), 8);
        assert_eq!(align_offset(2, 8), 8);
        assert_eq!(align_offset(3, 8), 8);
        assert_eq!(align_offset(4, 8), 8);
        assert_eq!(align_offset(5, 8), 8);
        assert_eq!(align_offset(6, 8), 8);
        assert_eq!(align_offset(7, 8), 8);
        assert_eq!(align_offset(8, 8), 8);

        assert_eq!(align_offset(1, 4), 4);
        assert_eq!(align_offset(2, 4), 4);
        assert_eq!(align_offset(3, 4), 4);
        assert_eq!(align_offset(4, 4), 4);

        assert_eq!(align_offset(0, 2), 0);
        assert_eq!(align_offset(1, 2), 2);
        assert_eq!(align_offset(2, 2), 2);
        assert_eq!(align_offset(3, 2), 4);

        assert_eq!(align_offset(0, 1), 0);
        assert_eq!(align_offset(1, 1), 1);
    }
}
