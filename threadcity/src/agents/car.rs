// auto comun

use super::{Agent, Vehicle};
use crate::model::Coord;

#[derive(Debug, Clone)]
pub struct Car {
    inner: Vehicle,
}

impl Car {
    pub fn new(id: u32, origin: (u32, u32), dest: (u32, u32)) -> Self {
        Self { inner: Vehicle::new(id, Coord::new(origin.0, origin.1), Coord::new(dest.0, dest.1)) }
    }
}

impl Agent for Car {
    fn id(&self) -> u32 { self.inner.id() }
    fn step(&mut self, dt_ms: u32) { self.inner.step(dt_ms); }
    fn pos(&self) -> Coord { self.inner.pos() }
}

