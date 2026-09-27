"""Module docstring explaining the purpose {kept}."""


def scan(target):
    # Skip when the target is missing.
    if not target.exists():
        return None

    # The heavy work happens after the guard above.
    return target.read_text()
