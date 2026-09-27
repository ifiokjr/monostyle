fn main() {
    let bytes = b"{ } brackets in bytes";
    let first = b'x';
    let marker = b"END";

    // A byte string with a quote inside.
    let quoted = b"say \"hi\"";

    println!("{:?} {:?} {:?} {:?}", bytes, first, marker, quoted);
}
