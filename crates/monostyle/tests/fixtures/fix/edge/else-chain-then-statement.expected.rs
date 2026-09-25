fn work(n: i32, base: i32) -> i32 {
    if n < 0 {
        0
    } else if n < 10 {
        n
    } else {
        10
    }

    base
}
