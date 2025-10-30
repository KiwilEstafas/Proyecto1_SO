// camion de carga que entrega insumos a la planta

use super::{Agent, Vehicle};
use crate::model::{Coord, SupplyKind};

#[derive(Debug, Clone)]
pub struct CargoTruck {
    inner: Vehicle,
    pub cargo: SupplyKind,
}

impl CargoTruck {
    pub fn new(id: u32, origin: (u32, u32), dest: (u32, u32), cargo: SupplyKind) -> Self {
        Self {
            inner: Vehicle::new(id, Coord::new(origin.0, origin.1), Coord::new(dest.0, dest.1)),
            cargo,
        }
    }
}

impl Agent for CargoTruck {
    fn id(&self) -> u32 { self.inner.id() }
    fn step(&mut self, dt_ms: u32) { self.inner.step(dt_ms); }
    fn pos(&self) -> Coord { self.inner.pos() }
    fn set_pos(&mut self, new_pos: Coord) {
        self.inner.set_pos(new_pos);}
    fn priority(&self) -> u8 { self.inner.priority }
}

