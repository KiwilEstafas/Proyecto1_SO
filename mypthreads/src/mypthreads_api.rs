use crate::api_context;
use crate::channels::SimpleMutex;
use crate::runtime::ThreadRuntimeV2;
use crate::signals::ThreadSignal;
use crate::thread::{ContextThreadEntry, SchedulerType, ThreadId};
use std::sync::atomic::{AtomicBool, Ordering};


//RUNTIME GLOBAL 
// Debe inicializarse explícitamente con `runtime_init()` antes de usar.
pub static mut RUNTIME: Option<(SimpleMutex, ThreadRuntimeV2)> = None;

/// Inicializa el runtime global de mypthreads.
/// Debe llamarse una sola vez!!!
pub fn runtime_init() {
    unsafe {
        if RUNTIME.is_none() {
            RUNTIME = Some((SimpleMutex::new(), ThreadRuntimeV2::new()));
        }
    }
}

/// Helper interno para obtener acceso mutable al runtime global.
fn get_runtime_mut() -> &'static mut (SimpleMutex, ThreadRuntimeV2) {
    unsafe {
        match RUNTIME.as_mut() {
            Some(t) => t,
            None => panic!("RUNTIME no inicializado: llamá a runtime_init() antes de usar la API"),
        }
    }
}

/// Tipos de parámetros de planificación (scheduler)
pub enum SchedulerParams {
    RoundRobin,
    Lottery { tickets: u32 },
    RealTime { deadline: u64 },
}

/// Crea un nuevo hilo manejado por mypthreads.
pub fn my_thread_create(
    name: &str,
    params: SchedulerParams,
    entry: ContextThreadEntry,
) -> ThreadId {
    let r = get_runtime_mut();
    let (mutex, runtime) = &mut *r;

    let current_tid = api_context::try_current_tid().unwrap_or(0);
    mutex.lock(current_tid);

    let (sched, tickets, deadline) = match params {
        SchedulerParams::RoundRobin => (SchedulerType::RoundRobin, 1, None),
        SchedulerParams::Lottery { tickets } => (SchedulerType::Lottery, tickets, None),
        SchedulerParams::RealTime { deadline } => (SchedulerType::RealTime, 0, Some(deadline)),
    };

    let id = runtime.spawn(name, sched, entry, tickets, deadline);

    mutex.unlock(current_tid);
    id
}

/// Marca un hilo como "detached"
pub fn my_thread_detach(tid: ThreadId) {
    let r = get_runtime_mut();
    let (mutex, runtime) = &mut *r;
    let current_tid = api_context::try_current_tid().unwrap_or(0);

    mutex.lock(current_tid);
    if let Some(thread) = runtime.threads.get_mut(&tid) {
        thread.detached = true;
    }
    mutex.unlock(current_tid);
}

/// Cambia los parámetros de planificación de un hilo
pub fn my_thread_chsched(tid: ThreadId, params: SchedulerParams) {
    let r = get_runtime_mut();
    let (mutex, runtime) = &mut *r;
    let current_tid = api_context::try_current_tid().unwrap_or(0);

    mutex.lock(current_tid);
    if let Some(thread) = runtime.threads.get_mut(&tid) {
        let (sched, tickets, deadline) = match params {
            SchedulerParams::RoundRobin => (SchedulerType::RoundRobin, 1, None),
            SchedulerParams::Lottery { tickets } => (SchedulerType::Lottery, tickets, None),
            SchedulerParams::RealTime { deadline } => (SchedulerType::RealTime, 0, Some(deadline)),
        };

        thread.sched_type = sched;
        thread.tickets = tickets;
        thread.deadline = deadline;
    }
    mutex.unlock(current_tid);
}

/// Ejecuta el runtime por una cantidad de ciclos simulados
pub fn run_simulation(cycles: usize) {
    let r = get_runtime_mut();
    let (mutex, runtime) = &mut *r;
    let tid = api_context::try_current_tid().unwrap_or(0);

    mutex.lock(tid);
    runtime.run(cycles);
    mutex.unlock(tid);
}

/// Desbloquea todos los hilos
pub fn runtime_unblock_all() {
    let r = get_runtime_mut();
    let (_mutex, runtime) = &mut *r;
    runtime.unblock_all_threads();
}

/// Ejecuta el scheduler por `cycles` ciclos
pub fn runtime_run_cycles(cycles: usize) {
    let r = get_runtime_mut();
    let (_mutex, runtime) = &mut *r;
    runtime.run(cycles);
}


pub struct MyMutex {
    locked: AtomicBool,
}

impl MyMutex {
    pub fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    /// Forzado para desbloquear (solo debe usarlo el hilo `main`)
    pub fn force_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

/// Inicializa un nuevo mutex cooperativo
pub fn my_mutex_init() -> MyMutex {
    MyMutex::new()
}

/// Intenta adquirir el lock cooperativamente
pub fn my_mutex_lock(mtx: &MyMutex) -> ThreadSignal {
    if mtx.locked.swap(true, Ordering::Acquire) {
        // Ya estaba bloqueado, entonces devolvemos señal para que el runtime pause el hilo
        ThreadSignal::MutexLock(mtx as *const _ as usize)
    } else {
        // Lock adquirido inmediatamente
        ThreadSignal::Continue
    }
}

/// Intenta adquirir el lock sin bloquearse.
pub fn my_mutex_trylock(mtx: &MyMutex) -> bool {
    !mtx.locked.swap(true, Ordering::Acquire)
}

/// Libera el lock.
pub fn my_mutex_unlock(mtx: &MyMutex) -> ThreadSignal {
    mtx.locked.store(false, Ordering::Release);
    ThreadSignal::Continue
}

/// Destruye el mutex (no hace nada porque no hay recursos dinámicos)
pub fn my_mutex_destroy(_mtx: &mut MyMutex) {}
