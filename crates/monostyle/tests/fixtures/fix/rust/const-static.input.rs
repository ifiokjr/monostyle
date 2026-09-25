use std::sync::atomic::AtomicU8;

pub const LIMIT: u8 = 64;
pub static COUNTER: AtomicU8 = AtomicU8::new(0);

const TABLE: [u8; 4] = [1, 2, 3, 4];


fn main() {
    let first = TABLE[0];


    println!("{first} {LIMIT}");
}
