fn search(grid: &[Vec<u8>], needle: u8) -> Option<(usize, usize)> {
    'outer: for (row, cells) in grid.iter().enumerate() {
        for (col, cell) in cells.iter().enumerate() {
            if *cell == needle {
                break 'outer;
            }
        }
    }


    None
}
