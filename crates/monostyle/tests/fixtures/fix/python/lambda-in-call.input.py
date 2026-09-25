def ordering(items):
    ordered = sorted(items, key=lambda item: (item[1], -item[0]))


    return ordered
