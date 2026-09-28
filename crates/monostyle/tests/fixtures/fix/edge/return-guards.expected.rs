fn parse(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }

    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    s.parse().ok()
}
