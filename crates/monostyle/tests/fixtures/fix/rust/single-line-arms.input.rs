fn label(n: i32) -> &'static str {
    match n {
        0 => "zero",
        1 => "one",
        _ => "many",
    }
}

fn guarded(n: i32) -> &'static str {
    match n {
        n if n < 0 => "negative",
        0..=9 => "small",
        _ => "large",
    }
}
