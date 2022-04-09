/// Perform the djb2 hash function
pub fn djb_hash(key: &str) -> u32 {
    let mut hash_value: u32 = 5381;
    for char in key.bytes() {
        hash_value = hash_value.wrapping_mul(33).wrapping_add(char as u32);
    }

    hash_value
}
