def handle(command):
    match command:
        case "go" | "run":
            return "moving"
        case {"key": value}:
            return f"key is {value}"
        case [first, *rest]:
            return f"first of {rest}"
        case _:
            return "unknown"
