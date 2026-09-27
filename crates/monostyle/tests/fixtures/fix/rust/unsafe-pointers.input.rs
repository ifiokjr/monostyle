fn sum_raw(values: *const u8, length: usize) -> u8 {
    let mut total = 0u8;

    unsafe {
        for offset in 0..length {
            total = total.wrapping_add(*values.add(offset));
        }
    }

    total
}

fn main() {
    let values = [1u8, 2, 3];


    println!("{}", sum_raw(values.as_ptr(), values.len()));
}
