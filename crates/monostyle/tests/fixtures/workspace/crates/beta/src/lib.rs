//! Beta crate.

pub fn multiply(a: i32, b: i32) -> i32 {
    if a == 0 {
        return 0;
    }
    if b == 0 {
        return 0;
    }
    let result = a * b;
    result
}
