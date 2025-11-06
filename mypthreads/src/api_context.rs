//! api de contexto para hilos preemptivos
//! los hilos usan esta api en lugar de acceder al runtime

use crate::thread::ThreadId;
use crate::signals::ThreadSignal;
use crate::channels::{ThreadChannels, JoinHandle, SimpleMutex};

// Thread-local storage para que cada hilo sepa su tid y tenga acceso a los canales
thread_local! {
    static CURRENT_TID: std::cell::RefCell<Option<ThreadId>> = std::cell::RefCell::new(None);
    static CHANNELS: std::cell::RefCell<Option<ThreadChannels>> = std::cell::RefCell::new(None);
}

/// inicializa el contexto del hilo actual
/// esto lo llama el wrapper al iniciar un hilo
pub fn init_thread_context(tid: ThreadId, channels: ThreadChannels) {
    CURRENT_TID.with(|t| *t.borrow_mut() = Some(tid));
    CHANNELS.with(|c| *c.borrow_mut() = Some(channels));
}

/// obtiene el tid del hilo actual
pub fn current_tid() -> ThreadId {
    CURRENT_TID.with(|t| {
        t.borrow().expect("hilo no inicializado")
    })
}

/// obtiene los canales del hilo actual
fn channels() -> ThreadChannels {
    CHANNELS.with(|c| {
        c.borrow().clone().expect("canales no inicializados")
    })
}

/// el hilo cede el control (yield)
pub fn ctx_yield() -> ThreadSignal {
    let tid = current_tid();
    channels().report_yield(tid);
    ThreadSignal::Yield
}

/// el hilo termina
pub fn ctx_exit() -> ThreadSignal {
    let tid = current_tid();
    channels().report_exit(tid);
    ThreadSignal::Exit
}

/// el hilo se bloquea
pub fn ctx_block() -> ThreadSignal {
    let tid = current_tid();
    channels().report_block(tid);
    ThreadSignal::Block
}

/// intenta hacer join a otro hilo
/// retorna true si el hilo ya terminó, false si debe bloquearse
pub fn ctx_join(join_handle: &JoinHandle) -> ThreadSignal {
    if join_handle.is_terminated() {
        ThreadSignal::Yield
    } else {
        ctx_block()
    }
}

/// intenta adquirir un mutex
pub fn ctx_mutex_lock(mutex: &SimpleMutex) -> ThreadSignal {
    let tid = current_tid();
    if mutex.lock(tid) {
        // debe bloquearse
        ctx_block()
    } else {
        // adquirió el lock
        ThreadSignal::Continue
    }
}

/// libera un mutex
pub fn ctx_mutex_unlock(mutex: &SimpleMutex) -> ThreadSignal {
    let tid = current_tid();
    if let Some(next_tid) = mutex.unlock(tid) {
        // hay un hilo esperando, reportarlo como ready
        // TODO: necesitamos una forma de despertar hilos
        // por ahora solo liberamos
        let _ = next_tid;
    }
    ThreadSignal::Continue
}

/// intenta adquirir un mutex sin bloquearse
pub fn ctx_mutex_trylock(mutex: &SimpleMutex) -> bool {
    let tid = current_tid();
    mutex.try_lock(tid)
}
