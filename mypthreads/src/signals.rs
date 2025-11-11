//! enum de senales que emiten los hilos

use crate::thread::ThreadId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadSignal {
    Continue, // hilo sigue y se reencola como yield en este mvp
    Yield,    // hilo cede y se reencola al final
    Block,    // hilo se bloquea y no se reencola
    Exit,     // hilo termina
    Join(ThreadId),
    MutexLock(usize),   
    MutexUnlock(usize), 
}

