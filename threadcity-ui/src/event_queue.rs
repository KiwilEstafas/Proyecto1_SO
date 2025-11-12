use std::collections::VecDeque

// representa un evento del simulador que el ui puede animar
#[derive(Clone, Debug)]
pub enum UiEvent {
    MoveEntity { id: usize, from: (i32, i32), to: (i32, i32) },
    SpawnEntity { id: usize, position: (i32, i32) },
    RemoveEntity { id: usize },
    Log(String),
}

// cola simple para almacenar los eventos generados
pub struct EventQueue {
    queue: VecDeque<UiEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self { queue: VecDeque::new() }
    }

    // agrega un evento al final de la cola
    pub fn push(&mut self, event: UiEvent) {
        self.queue.push_back(event)
    }

    // obtiene el siguiente evento si existe
    pub fn pop(&mut self) -> Option<UiEvent> {
        self.queue.pop_front()
    }

    // devuelve true si aun hay eventos por procesar
    pub fn has_events(&self) -> bool {
        !self.queue.is_empty()
    }
}
