mod kitchen {
    pub mod appliances {
        pub fn blend() -> &'static str {
            "blended"
        }
    }

    pub use self::appliances::blend;
}

fn main() {
    let result = kitchen::blend();

    println!("{result}");
}
