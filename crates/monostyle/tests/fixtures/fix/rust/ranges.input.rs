fn samples() {
    let numbers: Vec<i32> = (0..=10).step_by(2).collect();
    let head = &numbers[..3];
    let tail = &numbers[3..];
    let everything = &numbers[..];


    println!("{:?} {:?} {:?}", head, tail, everything);
}
