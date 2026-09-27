async fn fetch(key: &str) -> String {
    format!("value-for-{key}")
}

async fn load_all(keys: &[&str]) -> Vec<String> {
    let mut loaded = Vec::new();

    for key in keys {
        loaded.push(fetch(key).await);
    }

    loaded
}
