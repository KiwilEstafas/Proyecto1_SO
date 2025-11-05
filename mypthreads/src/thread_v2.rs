//! version 2 de thread con soporte para cambio de contexto real
//! usa ContextThreadEntry que no necesita &mut Runtime porque me dio problemitas jj

use crate::context_wrapper::ThreadContext;
use crate::signals::ThreadSignal;

pub type ThreadId = u32;

// nuevo tipo de entry sin acceso al runtime
pub type ContextThreadEntry = Box<dyn FnMut(ThreadId) -> ThreadSignal + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerType {
    RoundRobin,
    Lottery,
    RealTime,
}

// version 2 del hilo con context obligatorio
pub struct MyThreadV2 {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    pub tickets: u32,
    pub deadline: Option<u64>,
    pub detached: bool,
    pub joiners: Vec<ThreadId>,
    pub context: ThreadContext,
    
    // guardamos el entry aqui para poder ejecutarlo
    // pero NO lo exponemos al runtime
    entry: Option<ContextThreadEntry>,
}

impl MyThreadV2 {
    pub fn new(
        id: ThreadId,
        name: String,
        sched_type: SchedulerType,
        tickets: u32,
        deadline: Option<u64>,
        entry: ContextThreadEntry,
    ) -> Self {
        // crear el context con el wrapper
        let context = ThreadContext::new(thread_entry_wrapper);
        
        Self {
            id,
            name,
            state: ThreadState::New,
            sched_type,
            tickets,
            deadline,
            detached: false,
            joiners: Vec::new(),
            context,
            entry: Some(entry),
        }
    }
    
    // ejecutar un paso del hilo
    // esto lo llama el wrapper, no el runtime
    pub(crate) fn execute_step(&mut self) -> ThreadSignal {
        if let Some(ref mut entry) = self.entry {
            entry(self.id)
        } else {
            ThreadSignal::Exit
        }
    }
}

// PLACEHOLDER: esto lo implementaremos en PASO 2
extern "C" fn thread_entry_wrapper(_transfer: context::Transfer) -> ! {
    // TODO: implementar en PASO 2
    loop {
        std::hint::spin_loop();
    }
}
