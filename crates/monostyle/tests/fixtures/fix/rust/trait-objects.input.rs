type Handler = Box<dyn Fn(u8) -> u8>;

fn with_handlers(mut handlers: Vec<Handler>) -> Vec<u8> {
    let results: Vec<u8> = handlers
        .drain(..)
        .map(|handler| handler(7))
        .filter(|value| value % 2 == 0)
        .collect();


    results
}

fn main() {
    let handlers: Vec<Handler> = vec![Box::new(|n| n + 1)];


    println!("{:?}", with_handlers(handlers));
}
