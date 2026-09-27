macro_rules! sums {
    ($($value:expr),*) => {
        {
            let mut total = 0;
            $(total += $value;)*
            total
        }
    };
}

fn main() {
    let total = sums![1, 2, 3];

    println!("{total}");
}
