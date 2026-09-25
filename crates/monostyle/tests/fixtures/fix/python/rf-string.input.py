def pattern(name):
    raw = rf"^\s*{name}\s*$"
    combined = rf"{name} and {{kept}} \d+"


    return raw, combined
