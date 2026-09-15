#[derive(Clone, PartialEq, Message)]
pub struct GeneratedMessage {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
impl GeneratedMessage {
    pub fn new() -> Self { if true { Self { name: String::new() } } else { Self { name: String::new() } } }
}
