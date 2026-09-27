type Vector = list[float]
type Matrix = list[Vector]


def scale(matrix: Matrix, factor: float) -> Matrix:
    return [[value * factor for value in row] for row in matrix]
