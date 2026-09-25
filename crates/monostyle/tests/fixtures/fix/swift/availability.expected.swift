func feature() -> String {
    if #available(iOS 15, *) {
        return "modern"
    } else {
        return "legacy"
    }
}
