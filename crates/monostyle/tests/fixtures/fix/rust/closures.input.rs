fn pipeline(base: i32) -> Vec<i32> {
    let bump = move |x: i32| -> i32 { x + base };
    let combine = |a: i32, b: i32| a * 10 + b;
    let squares: Vec<i32> = (0..4).map(|n| n * n).collect();
    let folded = squares.iter().fold(0, |acc, n| combine(acc, *n) + bump(0));


    squares.into_iter().chain(std::iter::once(folded)).collect()
}
