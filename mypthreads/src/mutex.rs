//! primitiva minima de mutex con init y destroy
//! se extendera con lock unlock y trylock

use std::collections::VecDeque;
use crate::thread::ThreadId;

pub struct MyMutex {
    pub(crate) initialized: bool,
    pub(crate) locked: bool,
    pub(crate) _owner: Option<ThreadId>, // guion bajo indica que aun no se usa
    pub(crate) wait_queue: VecDeque<ThreadId>,
}

impl MyMutex {
    // inicializa el mutex
    pub fn my_mutex_init() -> Self {
        Self {
            initialized: true,
            locked: false,
            _owner: None,
            wait_queue: VecDeque::new(),
        }
    }

    // destruye el mutex
    // devuelve 0 si exito y 1 si falla por estado invalido
    pub fn my_mutex_destroy(&mut self) -> i32 {
        if !self.initialized {
            return 1;
        }
        if self.locked {
            return 1;
        }
        if !self.wait_queue.is_empty() {
            return 1;
        }
        self.initialized = false;
        0
    }
}

