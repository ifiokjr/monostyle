struct Point {
    x: i32,
    y: i32,
}

fn origin(point: Point) -> bool {
    let Point { x, y, .. } = point;
    let deep = match point {
        Point { x: 0, .. } => x,
        Point { y: 0, .. } => y,
        Point { x, y } => x + y,
    };

    deep == 0
}
