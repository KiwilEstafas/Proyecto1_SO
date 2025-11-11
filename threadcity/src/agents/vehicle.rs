use crate::model::Coord;
use std::any::Any;
use mypthreads::thread::ThreadId; 
use crate::model::SupplyKind;

// interfaz base de cualquier agente
pub trait Agent {
    fn id(&self) -> u32;
    fn step(&mut self, dt_ms: u32);
    fn pos(&self) -> Coord;
    fn set_pos(&mut self, new_pos: Coord);
    fn priority(&self) -> u8; 
}

pub trait AgentDowncast: Agent {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Agent + Any + 'static> AgentDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Traveling,
    WaitingForBridge,
    CrossingBridge,
    Arrived,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Car,
    Ambulance,
    Boat,
    CargoTruck(SupplyKind),
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id: u32,
    pub tid: ThreadId,
    pub pos: Coord,
    pub origin: Coord,
    pub destination: Coord,
    pub speed: f32,
    pub priority: u8,
    pub stage: AgentState
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub vehicle: Vehicle,
    pub agent_type: AgentType,
}

impl Vehicle {
    pub fn new(id: u32, tid: ThreadId, origin: Coord, destination: Coord) -> Self {
        Self {
            id,
            tid,
            pos: origin,
            origin,
            destination,
            speed: 1.0,
            priority: 0,
            stage: AgentState::Traveling
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
    fn set_pos(&mut self, new_pos: Coord) {
        self.pos = new_pos;}
    fn priority(&self) -> u8 { self.priority }
}

