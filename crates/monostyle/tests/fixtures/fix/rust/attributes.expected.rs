#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[repr(u8)]
pub enum Mode {
    Idle,
    Busy,
}

#[derive(Debug)]
#[must_use]
pub struct Config {
    pub retries: u8,
    pub mode: Mode,
}

fn main() {
    let config = Config { retries: 3, mode: Mode::Busy };

    println!("{config:?}");
}
