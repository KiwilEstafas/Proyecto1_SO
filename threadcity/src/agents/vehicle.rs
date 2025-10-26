use crate::model::Coord;
use std::any::Any;

// interfaz base de cualquier agente
pub trait Agent {
    fn id(&self) -> u32;
    fn step(&mut self, dt_ms: u32);
    fn pos(&self) -> Coord;
}

// helper para poder hacer downcast desde Box<dyn Agent>
pub trait AgentDowncast: Agent {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Agent + Any + 'static> AgentDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id: u32,
    pub pos: Coord,
    pub origin: Coord,
    pub destination: Coord,
    pub speed: f32,
    pub priority: u8,
}

impl Vehicle {
    pub fn new(id: u32, origin: Coord, destination: Coord) -> Self {
        Self {
            id,
            pos: origin,
            origin,
            destination,
            speed: 1.0,
            priority: 0,
        }
    }

    pub fn move_one(&mut self) {
        if self.pos.x < self.destination.x { self.pos.x += 1; }
        else if self.pos.x > self.destination.x { self.pos.x -= 1; }
        else if self.pos.y < self.destination.y { self.pos.y += 1; }
        else if self.pos.y > self.destination.y { self.pos.y -= 1; }
    }
}

impl Agent for Vehicle {
    fn id(&self) -> u32 { self.id }
    fn step(&mut self, _dt_ms: u32) { self.move_one(); }
    fn pos(&self) -> Coord { self.pos }
}

