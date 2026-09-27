type Shared = std::rc::Rc<std::cell::RefCell<Box<(dyn Fn(u8) -> u8 + 'static)>>>;

fn register() -> Shared {
    let handler: Box<(dyn Fn(u8) -> u8 + 'static)> = Box::new(|n| n * 2);

    std::rc::Rc::new(std::cell::RefCell::new(handler))
}
