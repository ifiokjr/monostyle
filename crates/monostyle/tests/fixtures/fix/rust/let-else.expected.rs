fn parse(input: Option<&str>) -> u32 {
    let Some(text) = input else {
        return 0;
    };

    let Ok(value) = text.parse::<u32>() else {
        return 0;
    };

    value
}
