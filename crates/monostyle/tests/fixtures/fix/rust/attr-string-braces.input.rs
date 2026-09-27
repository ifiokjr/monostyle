#[doc = "config with {braces} inside"]
#[derive(Debug)]
pub struct Config {
    /// The retry count, capped at {max_retries}.
    pub retries: u8,
}


fn main() {
    let config = Config { retries: 3 };


    println!("{config:?}");
}
