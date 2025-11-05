//! canales de comunicacion entre hilos y runtime
//! permite que los hilos se comuniquen sin &mut Runtime

use crate::thread_v2::ThreadId;
use std::sync::{Arc, Mutex};
use std::collections::{VecDeque, HashMap};

/// canales de comunicacion del runtime
#[derive(Clone)]
pub struct ThreadChannels {
    /// hilos que hicieron yield y estan listos
    pub yield_queue: Arc<Mutex<VecDeque<ThreadId>>>,
    
    /// hilos que se bloquearon
    pub blocked_queue: Arc<Mutex<VecDeque<ThreadId>>>,
    
    /// hilos que terminaron
    pub terminated_queue: Arc<Mutex<VecDeque<ThreadId>>>,
    
    /// datos compartidos entre hilos
    pub shared_data: Arc<Mutex<HashMap<String, SharedData>>>,
}

/// datos que se pueden compartir entre hilos
#[derive(Clone)]
pub enum SharedData {
    Counter(Arc<Mutex<i32>>),
    Flag(Arc<Mutex<bool>>),
    JoinHandle(JoinHandle),
    SimpleMutex(SimpleMutex),
}

impl ThreadChannels {
    pub fn new() -> Self {
        Self {
            yield_queue: Arc::new(Mutex::new(VecDeque::new())),
            blocked_queue: Arc::new(Mutex::new(VecDeque::new())),
            terminated_queue: Arc::new(Mutex::new(VecDeque::new())),
            shared_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// un hilo reporta que quiere hacer yield
    pub fn report_yield(&self, tid: ThreadId) {
        self.yield_queue.lock().unwrap().push_back(tid);
    }

    /// un hilo reporta que se bloqueó
    pub fn report_block(&self, tid: ThreadId) {
        self.blocked_queue.lock().unwrap().push_back(tid);
    }

    /// un hilo reporta que terminó
    pub fn report_exit(&self, tid: ThreadId) {
        self.terminated_queue.lock().unwrap().push_back(tid);
    }
    
    /// guardar dato compartido
    pub fn store(&self, key: String, data: SharedData) {
        self.shared_data.lock().unwrap().insert(key, data);
    }
    
    /// obtener dato compartido
    pub fn get(&self, key: &str) -> Option<SharedData> {
        self.shared_data.lock().unwrap().get(key).cloned()
    }
}

/// handle para hacer join a un hilo
#[derive(Clone)]
pub struct JoinHandle {
    terminated: Arc<Mutex<bool>>,
}

impl JoinHandle {
    pub fn new() -> Self {
        Self {
            terminated: Arc::new(Mutex::new(false)),
        }
    }

    pub fn mark_terminated(&self) {
        *self.terminated.lock().unwrap() = true;
    }

    pub fn is_terminated(&self) -> bool {
        *self.terminated.lock().unwrap()
    }
}

/// mutex simple para sincronizacion entre hilos
#[derive(Clone)]
pub struct SimpleMutex {
    locked: Arc<Mutex<Option<ThreadId>>>,
    wait_queue: Arc<Mutex<VecDeque<ThreadId>>>,
}

impl SimpleMutex {
    pub fn new() -> Self {
        Self {
            locked: Arc::new(Mutex::new(None)),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// intenta adquirir el lock
    /// retorna true si se adquirió, false si ya estaba tomado
    pub fn try_lock(&self, tid: ThreadId) -> bool {
        let mut locked = self.locked.lock().unwrap();
        if locked.is_none() {
            *locked = Some(tid);
            true
        } else {
            false
        }
    }
    
    /// adquiere el lock o se encola
    /// retorna true si debe bloquearse
    pub fn lock(&self, tid: ThreadId) -> bool {
        let mut locked = self.locked.lock().unwrap();
        if locked.is_none() {
            *locked = Some(tid);
            false // no se bloquea, adquirio el lock
        } else {
            // ya esta tomado, encolarse
            drop(locked);
            self.wait_queue.lock().unwrap().push_back(tid);
            true // debe bloquearse
        }
    }

    /// libera el lock
    /// retorna el siguiente hilo en espera si hay alguno
    pub fn unlock(&self, tid: ThreadId) -> Option<ThreadId> {
        let mut locked = self.locked.lock().unwrap();
        
        if *locked != Some(tid) {
            panic!("hilo {:?} intenta liberar mutex que no posee", tid);
        }
        
        *locked = None;
        drop(locked);
        
        // despertar al siguiente en la cola
        self.wait_queue.lock().unwrap().pop_front()
    }
}
