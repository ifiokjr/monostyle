def where(point):
    match point:
        case Point(x=0, y=0):
            return "origin"

        case Point(x=0):
            return "y-axis"

        case _:
            return "elsewhere"
