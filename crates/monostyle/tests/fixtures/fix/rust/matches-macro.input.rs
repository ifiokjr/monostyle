fn classify(value: Option<u8>) -> &'static str {
    if matches!(value, Some(0)) {
        "zero"
    } else if matches!(value, Some(n) if n > 10) {
        "many"
    } else {
        "other"
    }
}
