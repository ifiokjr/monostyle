fn adder(base: u8) -> impl Fn(u8) -> u8 {
    move |x| x.wrapping_add(base)
}

fn compose(f: impl Fn(u8) -> u8, g: impl Fn(u8) -> u8) -> impl Fn(u8) -> u8 {
    move |x| g(f(x))
}
