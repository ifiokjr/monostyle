fn main() {
    let pattern = r#"contains "quotes" and {braces}"#;
    let heavier = r##"has "quotes" and " "# inside"##;
    let tags = r#"<a href="link">{text}</a>"#;


    println!("{pattern} {heavier} {tags}");
}
