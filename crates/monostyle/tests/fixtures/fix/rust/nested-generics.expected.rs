use std::collections::HashMap;

fn deep(map: HashMap<String, Vec<Option<u8>>>) -> Vec<Vec<u8>> {
    let parsed: Result<u32, _> = "41".parse::<u32>();
    let bump = parsed.unwrap_or(0) as u8;
    let widened: Vec<Vec<u8>> = map
        .values()
        .map(|inner| {
            inner
                .iter()
                .filter_map(|slot| slot.as_ref().map(|byte| byte.wrapping_add(bump)))
                .collect()
        })
        .collect();

    widened
}
