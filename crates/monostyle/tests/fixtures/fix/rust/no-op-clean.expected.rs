fn tally(scores: &[u8]) -> u32 {
    let json = r#"{"kept": true, "n": 42}"#;

    scores
        .iter()
        .filter(|score| **score > 0)
        .map(|score| u32::from(*score))
        .sum::<u32>()
        + json.len() as u32
}

fn main() {
    let scores = [3, 0, 9];

    println!("{}", tally(&scores));
}
