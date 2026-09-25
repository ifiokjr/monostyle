def matrix(rows, cols):
    grid = {(r, c): r * cols + c for r in range(rows) for c in range(cols) if r != c}
    evens = [value for row in grid.values() for value in [row] if value % 2 == 0]


    return evens
