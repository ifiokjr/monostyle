fn payload() -> &'static str {
    let json = r#"
{
  "user": "ada",
  "roles": ["admin"],
  "meta": { "active": true }
}
"#;

    // A second raw body with unbalanced-looking content.
    let template = r#"
if (ready) {
    go("now");
}
"#;

    Box::leak(format!("{json}{template}").into_boxed_str())
}
