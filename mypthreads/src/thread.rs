//! version 2 de thread con soporte para cambio de contexto real

use crate::context_wrapper::ThreadContext;
use crate::signals::ThreadSignal;
use crate::thread_data::{ThreadGlobalContext, ThreadResponse, TransferMessage};
use crate::JoinHandle;
use context::Transfer;

pub type ThreadId = u32;

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

pub struct MyThread {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    pub tickets: u32,
    pub deadline: Option<u64>,
    pub detached: bool,
    pub joiners: Vec<ThreadId>,
    pub join_handle: JoinHandle, //Para saber quienes estan esperando por el hilo
    pub context: ThreadContext,
    entry: Option<ContextThreadEntry>,
}

impl MyThread {
    pub fn new(
        id: ThreadId,
        name: String,
        sched_type: SchedulerType,
        tickets: u32,
        deadline: Option<u64>,
        entry: ContextThreadEntry,
    ) -> Self {
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
            join_handle: JoinHandle::new(),
            context,
            entry: Some(entry),
        }
    }

    /// ejecutar un paso del hilo
    pub(crate) fn execute_step(&mut self) -> ThreadSignal {
        if let Some(ref mut entry) = self.entry {
            entry(self.id)
        } else {
            ThreadSignal::Exit
        }
    }
}

/// EL WRAPPER REAL
/// ejecutamos el hilo y retornamos la respuesta via Transfer
extern "C" fn thread_entry_wrapper(mut transfer: Transfer) -> ! {
    // FASE 1: Inicialización
    let init_msg = unsafe { TransferMessage::unpack(transfer.data) };

    let (thread_ptr, channels, tid) = match init_msg {
        TransferMessage::Init {
            thread_ptr,
            channels,
            runtime_context_ptr: _,
        } => {
            let tid = unsafe { (*thread_ptr).id };
            (thread_ptr, channels, tid)
        }
        _ => {
            eprintln!("ERROR: thread_entry_wrapper esperaba mensaje Init");
            std::process::abort();
        }
    };

    // Inicializar contexto global
    ThreadGlobalContext::init(tid, channels.clone());

    // Inicializar API de contexto
    crate::api_context::init_thread_context(tid, channels);

    println!("[Hilo {}] inicializado correctamente", tid);

    // FASE 2: Loop de Ejecución
    loop {
        // Ejecutar un paso del hilo
        let signal = unsafe {
            let thread = &mut *thread_ptr;
            thread.execute_step()
        };

        println!("[Hilo {}] execute_step retornó: {:?}", tid, signal);

        // Convertir señal a respuesta
        let response = match signal {
            ThreadSignal::Yield | ThreadSignal::Continue => {
                println!("[Hilo {}] preparando yield al runtime", tid);
                ThreadResponse::Yield
            }

            ThreadSignal::Block => {
                println!("[Hilo {}] preparando block", tid);
                ThreadResponse::Block
            }
            ThreadSignal::Exit => {
                println!("[Hilo {}] preparando exit", tid);
                ThreadResponse::Exit
            }
            ThreadSignal::Join(target_tid) => ThreadResponse::Join(target_tid),
            ThreadSignal::MutexLock(mutex_addr) => ThreadResponse::MutexLock(mutex_addr),
            ThreadSignal::MutexUnlock(mutex_addr) => ThreadResponse::MutexUnlock(mutex_addr),
        };

        // Guardar si es Exit antes de mover response
        let is_exit = matches!(response, ThreadResponse::Exit);

        // Retornar al runtime empaquetando la respuesta
        let response_data = response.pack();

        // CLAVE: usamos el contexto que nos pasó el Transfer para retornar
        transfer = unsafe { transfer.context.resume(response_data) };

        // Cuando volvamos aquí, el runtime nos despertó
        println!("[Hilo {}] despertado por el runtime", tid);

        // Si la respuesta fue Exit, no deberíamos estar aquí
        if is_exit {
            eprintln!("[Hilo {}] ERROR: runtime despertó un hilo terminado", tid);
            std::process::abort();
        }
    }
}
