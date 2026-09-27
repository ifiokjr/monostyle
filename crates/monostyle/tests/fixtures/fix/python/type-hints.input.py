def index(users: dict[str, int]) -> list[str] | None:
    names: list[str] = [name for name, uid in users.items() if uid > 0]


    return names or None
