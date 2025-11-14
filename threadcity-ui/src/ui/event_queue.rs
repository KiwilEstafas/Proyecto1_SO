// cola fifo de eventos para animacion sin mutex ni hilos
// se usa solo en el hilo del ui

use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum EntityKind {
    Car,
    Ambulance,
    Boat,
    Truck,
}

#[derive(Clone, Debug)]
pub enum UiEvent {
    Spawn { id: u32, kind: EntityKind, pos: (u32, u32) },
    Move  { id: u32, to: (u32, u32) },
    Remove { id: u32 },
    Log(String),
    SimulationFinished
}

pub struct EventQueue {
    queue: VecDeque<UiEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    pub fn push(&mut self, ev: UiEvent) {
        self.queue.push_back(ev);
    }

    pub fn pop(&mut self) -> Option<UiEvent> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
