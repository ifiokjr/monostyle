/// Formats a value.
///
/// ```text
/// output = format!("{} of {}", done, total)
/// ```
///
/// # Errors
///
/// Returns an error when the buffer is full.
pub fn format_value(done: u8, total: u8) -> String {
    let header = format!("{done}/{total}");

    header
}
