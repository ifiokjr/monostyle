fn longest<'a, 'b>(first: &'a str, second: &'b str) -> &'a str
where
    'b: 'a,
{
    if first.len() > second.len() {
        first
    } else {
        second
    }
}

fn holder<'a>(text: &'a str) -> impl Fn() -> &'a str + 'a {
    move || text
}
