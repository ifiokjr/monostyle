fn patterns() {
    let tricky = r###"contains "# inside and "## too"###;
    let bytes = br#"raw bytes {kept} and "quotes""#;
    let bytes_hashed = br##"ends with "# here"##;


    println!("{tricky} {bytes} {bytes_hashed}");
}
