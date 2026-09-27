fn banner(name: &str) {
    let line = "line one\n";
    let continued = "the text goes on and on \
and continues here";
    let quoted = "she said \"hi\" and left";
    let escaped = "backslash \\";


    println!("{line}{continued}\n{quoted}\n{escaped}\n{name}");
}
