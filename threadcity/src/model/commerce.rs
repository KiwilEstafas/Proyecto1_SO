use super::Coord;

#[derive(Debug, Clone)]
pub struct Commerce {
    pub id: u32,
    pub location: Coord,
}

impl Commerce {
    pub fn new(id: u32, location: Coord) -> Self { Self { id, location } }
}

