const URL: &str = "https://example.com/a-very-long-path-that-exceeds-any-line-limit-with-no-break-point?query=1&more=2";

fn use_it() -> &'static str {
    URL
}
