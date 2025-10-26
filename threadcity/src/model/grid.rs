// grid  de la ciudad

#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub rows: u32,
    pub cols: u32,
}

impl Grid {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self { rows, cols }
    }
}

