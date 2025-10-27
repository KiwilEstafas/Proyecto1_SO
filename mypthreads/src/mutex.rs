//! primitiva minima de mutex con init y destroy
//! se extendera con lock unlock y trylock

use std::collections::VecDeque;
use crate::thread::ThreadId;

pub struct MyMutex {
    pub(crate) initialized: bool,
    pub(crate) locked: bool,
    pub(crate) owner: Option<ThreadId>, 
    pub(crate) wait_queue: VecDeque<ThreadId>,
}

impl MyMutex {
    // inicializa el mutex
    pub fn my_mutex_init() -> Self {
        Self {
            initialized: true,
            locked: false,
            owner: None,
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

    pub fn my_mutex_lock(&mut self, tid: ThreadId) -> bool {
        if !self.initialized{
            panic!("El mutex no se ha inicializado");
        }

        if self.locked{
            //Si ya esta bloqueado, se pone en la cola de espera
            if !self.wait_queue.contains(&tid){
                self.wait_queue.push_back(tid);
            }
            return true; //SI se debe bloquear
        }

        //SI el mutex esta libre, se adquiere
        self.locked = true;
        self.owner = Some(tid);
        false //No se bloquea
    }

    pub fn my_mutex_unlock(&mut self, tid:ThreadId) -> Option<ThreadId>{
        if !self.initialized {
            panic!("El mutex no ha sido inicializado");
        }

        if !self.locked{
            panic!("Error, el Mutex no esta bloqueado");
        }

        if self.owner != Some(tid){
            panic!("El hilo no es el dueño del mutex");
        }

        self.locked = false; 
        self.owner = None;
        self.wait_queue.pop_front()
    }

    pub fn my_mutex_trylock(&mut self, tid: ThreadId) -> bool {
        if !self.initialized {
            return false; 
        }

        if self.locked{
            return false; //Esta ocupado
        }

        self.locked = true; 
        self.owner = Some(tid);
        true
    }


}

