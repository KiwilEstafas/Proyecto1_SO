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

    /// tamano total de celdas del grid (rows * cols)
    #[inline]
    pub fn size(&self) -> u32 {
        self.rows.saturating_mul(self.cols)
    }
}
