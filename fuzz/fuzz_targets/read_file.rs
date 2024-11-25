#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};

fn fuzz_hash_table(table: &gvdb::read::HashTable, recursion_limit: usize) -> bool {
    let mut keep = false;

    for key in table.keys() {
        if let Ok(key) = key {
            keep = true;
            if let Ok(v) = table.get_value(&key) {
                v.value_signature();
                v.to_string();
            }

            if let Ok(ht) = table.get_hash_table(&key) {
                if recursion_limit > 0 {
                    keep &= fuzz_hash_table(&ht, recursion_limit - 1);
                }
            }
        }
    }

    keep
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Ok(file) = gvdb::read::File::from_bytes(data.into()) {
        if let Ok(table) = file.hash_table() {
            if fuzz_hash_table(&table, 3) {
                return Corpus::Keep;
            }
        }
    }

    Corpus::Reject
});
