struct Buffer<const N: usize> {
    data: [u8; N],
    used: usize,
}

impl<const N: usize> Buffer<N> {
    const fn new() -> Self {
        Self { data: [0; N], used: 0 }
    }

    fn push(&mut self, value: u8) {
        if self.used < N {
            self.data[self.used] = value;
            self.used += 1;
        }
    }

    const fn capacity() -> usize {
        N
    }
}

fn main() {
    let mut buffer = Buffer::<8>::new();


    buffer.push(1);
    println!("{} of {}", buffer.used, Buffer::<8>::capacity());
}
