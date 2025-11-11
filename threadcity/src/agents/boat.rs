// barco que se mueve sobre el rio

use super::{Agent, Vehicle};
use crate::model::Coord;
use mypthreads::ThreadId;

#[derive(Debug, Clone)]
pub struct Boat {
    pub inner: Vehicle,
}

impl Boat {
    pub fn new(id: u32, tid: ThreadId, origin: (u32, u32), dest: (u32, u32)) -> Self {
        Self { inner: Vehicle::new(id, tid, Coord::new(origin.0, origin.1), Coord::new(dest.0, dest.1)) }
    }
}

impl Agent for Boat {
    fn id(&self) -> u32 { self.inner.id() }
    fn step(&mut self, dt_ms: u32) { self.inner.step(dt_ms); }
    fn pos(&self) -> Coord { self.inner.pos() }
    fn set_pos(&mut self, new_pos: Coord) {
        self.inner.set_pos(new_pos);}
    fn priority(&self) -> u8 { self.inner.priority }
}

