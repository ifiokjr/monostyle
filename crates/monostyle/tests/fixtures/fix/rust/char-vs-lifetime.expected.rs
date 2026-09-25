fn separator<'a>(parts: &'a [&'a str], glue: char) -> String {
    let mut joined = String::new();
    let mut index = 0;

    for part in parts {
        if 0 < index {
            joined.push(glue);
        }

        joined.push_str(part);
        index += 1;
    }

    joined
}

fn main() {
    let text: &'static str = "a,b";
    let pieces = text.split(',').collect::<Vec<_>>();

    println!("{}", separator(&pieces, ';'));
}
