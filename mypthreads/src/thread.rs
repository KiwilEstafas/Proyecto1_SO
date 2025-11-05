//! tipos basicos y estructura del hilo

use crate::runtime::ThreadRuntime;
use crate::context_wrapper::ThreadContext;

// identificador de hilo
pub type ThreadId = u32;

// estados del hilo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
    Detached,
}

// tipos de planificador
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerType {
    RoundRobin,
    Lottery,
    RealTime,
}

// entry es el closure que ejecuta el hilo y devuelve una senal
pub type ThreadEntry =
    Box<dyn FnMut(&mut ThreadRuntime, ThreadId) -> crate::signals::ThreadSignal + 'static>;

pub struct RetVal(pub *mut core::ffi::c_void);
unsafe impl Send for RetVal {}
unsafe impl Sync for RetVal {}

// estructura del hilo
pub struct MyThread {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    pub tickets: u32, // para lottery
    pub deadline: Option<u64>, // deadline absoluto en milisegundos logicos
    pub detached: bool, // si es true no se puede hacer join y se limpia al terminar
    pub joiners: Vec<ThreadId>, // hilos que esperan a este hilo
    pub return_value: Option<RetVal>, // valor opaco para join estilo pthreads
    pub(crate) entry: Option<ThreadEntry>, // mantener por compatibilidad temporalmente
    pub context: Option<ThreadContext>, // nuevo: contexto para cambio de contexto real
}

impl MyThread {
    pub fn new(
        id: ThreadId,
        name: String,
        sched_type: SchedulerType,
        entry: ThreadEntry,
        tickets: u32,
        deadline: Option<u64>,
    ) -> Self {
        Self {
            id,
            name,
            state: ThreadState::New,
            sched_type,
            tickets,
            deadline,
            detached: false,
            joiners: Vec::new(),
            return_value: None,
            entry: Some(entry),
            context: None, // se inicializara en spawn cuando creemos el wrapper
        }
    }
}
