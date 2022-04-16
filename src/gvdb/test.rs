use pretty_assertions::assert_str_eq;
use std::cmp::{max, min};

fn write_byte_row(
    f: &mut dyn std::io::Write,
    offset: usize,
    bytes_per_row: usize,
    bytes: &[u8],
) -> std::io::Result<()> {
    write!(f, "{:08X}", offset)?;

    for (index, byte) in bytes.iter().enumerate() {
        if index % 4 == 0 {
            write!(f, " ")?;
        }

        write!(f, " {:02X}", byte)?;
    }

    let bytes_per_row = max(bytes_per_row, bytes.len());
    for index in bytes.len()..bytes_per_row {
        if index % 4 == 0 {
            write!(f, " ")?;
        }

        write!(f, "   ")?;
    }

    write!(f, "  ")?;

    for byte in bytes {
        if byte.is_ascii_alphanumeric() {
            write!(f, "{}", *byte as char)?;
        } else {
            write!(f, ".")?;
        }
    }

    writeln!(f)
}

fn write_byte_rows(
    f: &mut dyn std::io::Write,
    center_offset: usize,
    additional_rows_top: usize,
    additional_rows_bottom: usize,
    bytes_per_row: usize,
    bytes: &[u8],
) -> std::io::Result<()> {
    let center_row_num = center_offset / bytes_per_row;
    let start_row = center_row_num - min(center_row_num, additional_rows_top);
    // We add 1 because we can add partial rows at the end
    let last_row = min(
        additional_rows_bottom + center_row_num,
        bytes.len() / bytes_per_row + 1,
    );
    let row_count = last_row - start_row;

    for row in 0..row_count {
        let offset_start = (start_row + row) * bytes_per_row;
        let offset_end = min(bytes.len(), offset_start + bytes_per_row);

        write_byte_row(
            f,
            offset_start,
            bytes_per_row,
            &bytes[offset_start..offset_end],
        )?;
    }

    Ok(())
}

pub fn assert_bytes_eq(a: &[u8], b: &[u8], context: &str) {
    const WIDTH: usize = 16;
    const EXTRA_ROWS_TOP: usize = 8;
    const EXTRA_ROWS_BOTTOM: usize = 4;

    for (index, a_byte) in a.iter().enumerate() {
        let b_byte = b.get(index);

        if b_byte.is_none() || a_byte != b_byte.unwrap() {
            let mut a_bytes_buf = Vec::new();
            write_byte_rows(
                &mut a_bytes_buf,
                index,
                EXTRA_ROWS_TOP,
                EXTRA_ROWS_BOTTOM,
                WIDTH,
                &a,
            )
            .unwrap();
            let str_a = String::from_utf8(a_bytes_buf).unwrap();

            let mut b_bytes_buf = Vec::new();
            write_byte_rows(
                &mut b_bytes_buf,
                index,
                EXTRA_ROWS_TOP,
                EXTRA_ROWS_BOTTOM,
                WIDTH,
                &b,
            )
            .unwrap();
            let str_b = String::from_utf8(b_bytes_buf).unwrap();

            eprintln!("{}", context);
            assert_str_eq!(str_a, str_b);
        }
    }

    if a.len() > b.len() {
        eprintln!("{}", context);
        panic!("b is too big, expected {} bytes, got {}", a.len(), b.len());
    }
}
