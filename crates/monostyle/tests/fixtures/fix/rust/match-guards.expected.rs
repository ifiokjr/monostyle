fn classify(number: i32) -> &'static str {
    match number {
        n if n < 0 => "negative",
        0 | 1 | 2 => "small",
        n if n % 2 == 0 => "even",
        _ => "other",
    }
}

fn main() {
    for value in [-3, 1, 8, 7] {
        println!("{value} is {}", classify(value));
    }
}
