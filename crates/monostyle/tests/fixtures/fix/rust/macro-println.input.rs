fn report(name: &str, count: usize) {
    println!("report for {}: {} items", name, count);
    println!("{name:>8} -> {count:#010b}");
    let lines = vec!["a", "b"];
    let summary = format!("{} items in {:?}", count, lines);


    println!("{summary}");
}
