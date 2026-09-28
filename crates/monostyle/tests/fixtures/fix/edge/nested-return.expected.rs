fn work(deep: bool) -> u8 {
    if deep {
        let a = 1;
        let b = a + 1;
        return b;
    }

    0
}
