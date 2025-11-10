//! canales de comunicacion entre hilos y runtime
//! permite que los hilos se comuniquen sin &mut Runtime

use crate::thread::ThreadId;
use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const UNLOCKED: u32 = u32::MAX;

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

/// mutex simple para sincronizacion entre hilos, rediseñado con atomics.
#[derive(Clone)]
pub struct SimpleMutex {
    /// El estado del lock.
    /// Contiene UNLOCKED (0) si está libre, o el ThreadId del dueño si está tomado.
    pub owner: Arc<AtomicU32>,

    /// Cola de hilos esperando por el mutex.
    /// Usamos UnsafeCell porque la sincronización la provee el runtime.
    /// Solo el bucle del runtime (que es single-threaded) llama a .lock() y .unlock(),
    /// por lo que el acceso a la cola está serializado.
    pub wait_queue: Arc<UnsafeCell<VecDeque<ThreadId>>>,
}
unsafe impl Send for SimpleMutex {}
unsafe impl Sync for SimpleMutex {}
// Requerido porque UnsafeCell no es Sync por defecto.
// Garantizamos la seguridad porque el acceso a la cola está controlado por el runtime.

impl SimpleMutex {
    pub fn new() -> Self {
        Self {
            owner: Arc::new(AtomicU32::new(UNLOCKED)),
            wait_queue: Arc::new(UnsafeCell::new(VecDeque::new())),
        }
    }

    /// Intenta adquirir el lock de forma atómica y sin bloqueo.
    /// Retorna true si se adquirió, false si ya estaba tomado.
    /// Esta función es segura para ser llamada desde cualquier hilo en cualquier momento.
    pub fn try_lock(&self, tid: ThreadId) -> bool {
        // `compare_exchange` es una operación atómica que intenta cambiar el valor
        // de `owner` desde `UNLOCKED` a `tid`.
        // Si tiene éxito (el valor era `UNLOCKED`), devuelve Ok, y hemos adquirido el lock.
        // Si falla (el valor era otro), devuelve Err, y no hacemos nada.
        self.owner
            .compare_exchange(
                UNLOCKED,
                tid,
                Ordering::Acquire, // Barrera de memoria para asegurar que lecturas posteriores vean los cambios.
                Ordering::Relaxed, // Sin barrera si falla, no es necesario.
            )
            .is_ok()
    }

    /// Adquiere el lock o se encola si está ocupado.
    /// Esta función es llamada únicamente por el runtime.
    /// Retorna true si el hilo debe bloquearse.
    pub fn lock(&self, tid: ThreadId) -> bool {
        // Primero, intentamos tomar el lock de forma no bloqueante.
        if self.try_lock(tid) {
            // ¡Éxito! El lock se adquirió inmediatamente.
            false // No es necesario bloquearse.
        } else {
            // El lock está ocupado. Hay que encolar el hilo actual.
            // SAFETY: El acceso a wait_queue es seguro porque solo el runtime llama a esta función.
            let queue = unsafe { &mut *self.wait_queue.get() };
            queue.push_back(tid);
            true // Sí es necesario bloquearse.
        }
    }

    /// Libera el lock.
    /// Si hay un hilo esperando, la propiedad del lock se le transfiere directamente.
    /// Retorna el siguiente hilo en espera si hay alguno.
    /// Esta función es llamada únicamente por el runtime.
    pub fn unlock(&self, tid: ThreadId) -> Option<ThreadId> {
        let current_owner = self.owner.load(Ordering::Relaxed);
        if current_owner != tid {
            panic!(
                "hilo {:?} intenta liberar mutex que no posee (dueño actual: {:?})",
                tid, current_owner
            );
        }

        // SAFETY: El acceso a wait_queue es seguro por la misma razón que en lock().
        let queue = unsafe { &mut *self.wait_queue.get() };

        if let Some(next_tid) = queue.pop_front() {
            // Hay un hilo esperando. Le transferimos la propiedad del lock directamente.
            self.owner.store(next_tid, Ordering::Release);
            // Retornamos el ID del hilo para que el runtime lo despierte.
            Some(next_tid)
        } else {
            // No hay nadie en la cola de espera. Marcamos el lock como libre.
            self.owner.store(UNLOCKED, Ordering::Release);
            // No hay hilo que despertar.
            None
        }
    }

    /// Fuerza el unlock sin verificar ownership
    /// SOLO para usar desde el main thread en situaciones controladas
    pub fn force_unlock(&self) {
        // SAFETY: El acceso a wait_queue es seguro porque esto solo se llama desde main
        // cuando los demás threads están bloqueados
        let queue = unsafe { &mut *self.wait_queue.get() };

        if let Some(next_tid) = queue.pop_front() {
            // Hay un hilo esperando. Le transferimos la propiedad del lock directamente.
            self.owner.store(next_tid, Ordering::Release);
        } else {
            // No hay nadie en la cola de espera. Marcamos el lock como libre.
            self.owner.store(UNLOCKED, Ordering::Release);
        }
    }
}
