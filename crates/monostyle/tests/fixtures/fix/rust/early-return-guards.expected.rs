fn parse(input: &str) -> Option<u32> {
    if input.is_empty() {
        return None;
    }

    if !input.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let parsed: u32 = input.parse().ok()?;

    if parsed > 9000 {
        return None;
    }

    Some(parsed)
}
