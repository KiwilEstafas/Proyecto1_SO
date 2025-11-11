//! canales de comunicacion entre hilos y runtime
use crate::shared;
use crate::thread::ThreadId;
use crate::sync::{Shared};
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

const UNLOCKED: u32 = u32::MAX;

/// canales de comunicacion del runtime
#[derive(Clone)]
pub struct ThreadChannels {
    /// hilos que hicieron yield y estan listos
    pub yield_queue: Shared<VecDeque<ThreadId>>,

    /// hilos que se bloquearon
    pub blocked_queue: Shared<VecDeque<ThreadId>>,

    /// hilos que terminaron
    pub terminated_queue: Shared<VecDeque<ThreadId>>,

    /// datos compartidos entre hilos
    pub shared_data: Shared<HashMap<String, SharedData>>,
}

/// datos que se pueden compartir entre hilos
#[derive(Clone)]
pub enum SharedData {
    Counter(Shared<i32>),
    Flag(Shared<bool>),
    JoinHandle(JoinHandle),
    SimpleMutex(SimpleMutex),
}

impl ThreadChannels {
    pub fn new() -> Self {
        Self {
            yield_queue: shared(VecDeque::new()),
            blocked_queue: shared(VecDeque::new()),
            terminated_queue: shared(VecDeque::new()),
            shared_data: shared(HashMap::new()),
        }
    }

    /// un hilo reporta que quiere hacer yield
    pub fn report_yield(&self, tid: ThreadId) {
        if let Some(mut q) = self.yield_queue.try_enter() {
            q.push_back(tid);
            self.yield_queue.request_unlock();
        }
    }

    /// un hilo reporta que se bloqueó
    pub fn report_block(&self, tid: ThreadId) {
        if let Some(mut q) = self.blocked_queue.try_enter() {
            q.push_back(tid);
            self.blocked_queue.request_unlock();
        }
    }

    /// un hilo reporta que terminó
    pub fn report_exit(&self, tid: ThreadId) {
        if let Some(mut q) = self.terminated_queue.try_enter() {
            q.push_back(tid);
            self.terminated_queue.request_unlock();
        }
    }

    /// guardar dato compartido
    pub fn store(&self, key: String, data: SharedData) {
        if let Some(mut map) = self.shared_data.try_enter() {
            map.insert(key, data);
            self.shared_data.request_unlock();
        }
    }

    /// obtener dato compartido
    pub fn get(&self, key: &str) -> Option<SharedData> {
        if let Some(map) = self.shared_data.try_enter() {
            let result = map.get(key).cloned();
            self.shared_data.request_unlock();
            result
        } else {
            None
        }
    }
}

/// handle para hacer join a un hilo
#[derive(Clone)]
pub struct JoinHandle {
    terminated: Shared<bool>,
}

impl JoinHandle {
    pub fn new() -> Self {
        Self {
            terminated: shared(false),
        }
    }

    pub fn mark_terminated(&self) {
        if let Some(mut flag) = self.terminated.try_enter() {
            *flag = true;
            self.terminated.request_unlock();
        }
    }

    pub fn is_terminated(&self) -> bool {
        if let Some(flag) = self.terminated.try_enter() {
            let result = *flag;
            self.terminated.request_unlock();
            result
        } else {
            false
        }
    }
}

/// mutex simple para sincronizacion entre hilos
#[derive(Clone)]
pub struct SimpleMutex {
    /// El estado del lock.
    /// Contiene UNLOCKED (0) si está libre, o el ThreadId del dueño si está tomado.
    pub owner: Arc<AtomicU32>,

    /// Cola de hilos esperando por el mutex.
    pub wait_queue: Arc<UnsafeCell<VecDeque<ThreadId>>>,
}

unsafe impl Send for SimpleMutex {}
unsafe impl Sync for SimpleMutex {}

impl SimpleMutex {
    pub fn new() -> Self {
        Self {
            owner: Arc::new(AtomicU32::new(UNLOCKED)),
            wait_queue: Arc::new(UnsafeCell::new(VecDeque::new())),
        }
    }

    pub fn try_lock(&self, tid: ThreadId) -> bool {
        self.owner
            .compare_exchange(
                UNLOCKED,
                tid,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
    }

    pub fn lock(&self, tid: ThreadId) -> bool {
        if self.try_lock(tid) {
            false
        } else {
            let queue = unsafe { &mut *self.wait_queue.get() };
            queue.push_back(tid);
            true
        }
    }

    pub fn unlock(&self, tid: ThreadId) -> Option<ThreadId> {
        let current_owner = self.owner.load(Ordering::Relaxed);
        if current_owner != tid {
            panic!(
                "hilo {:?} intenta liberar mutex que no posee (dueño actual: {:?})",
                tid, current_owner
            );
        }

        let queue = unsafe { &mut *self.wait_queue.get() };

        if let Some(next_tid) = queue.pop_front() {
            self.owner.store(next_tid, Ordering::Release);
            Some(next_tid)
        } else {
            self.owner.store(UNLOCKED, Ordering::Release);
            None
        }
    }

    pub fn force_unlock(&self) {
        let queue = unsafe { &mut *self.wait_queue.get() };
        if let Some(next_tid) = queue.pop_front() {
            self.owner.store(next_tid, Ordering::Release);
        } else {
            self.owner.store(UNLOCKED, Ordering::Release);
        }
    }
}
