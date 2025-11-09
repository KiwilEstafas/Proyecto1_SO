//! version 2 de thread con soporte para cambio de contexto real

use crate::context_wrapper::ThreadContext;
use crate::signals::ThreadSignal;
use crate::thread_data::{ThreadGlobalContext, ThreadResponse, TransferMessage};
use crate::JoinHandle;
use context::Transfer;

pub type ThreadId = u32;

// --- CAMBIO: La clausura ahora acepta (tid, tickets) ---
pub type ContextThreadEntry = Box<dyn FnMut(ThreadId, u32) -> ThreadSignal + Send + 'static>;

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
    pub join_handle: JoinHandle,
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

    /// ejecutar un paso del hilo, pasando los tiquetes actuales
    pub(crate) fn execute_step(&mut self, current_tickets: u32) -> ThreadSignal {
        if let Some(ref mut entry) = self.entry {
            entry(self.id, current_tickets)
        } else {
            ThreadSignal::Exit
        }
    }
}

/// EL WRAPPER REAL
/// Se ejecuta en la pila del nuevo hilo y maneja la comunicación con el Runtime.
extern "C" fn thread_entry_wrapper(mut transfer: Transfer) -> ! {
    // FASE 1: Inicialización
    // Desempacamos el mensaje inicial que nos envió el Runtime
    let (thread_ptr, channels, tid, mut current_tickets) =
        if let TransferMessage::Init {
            thread_ptr,
            channels,
            runtime_context_ptr: _,
            current_tickets,
        } = unsafe { TransferMessage::unpack(transfer.data) }
        {
            let tid = unsafe { (*thread_ptr).id };
            (thread_ptr, channels, tid, current_tickets)
        } else {
            eprintln!("ERROR: thread_entry_wrapper esperaba mensaje Init");
            std::process::abort();
        };

    // Inicializar contextos para que las APIs funcionen
    ThreadGlobalContext::init(tid, channels.clone());
    crate::api_context::init_thread_context(tid, channels);

    println!("[Hilo {}] inicializado correctamente", tid);

    // FASE 2: Loop de Ejecución
    loop {
        // Ejecutar un paso de la lógica del hilo (ej. vehicle_logic)
        // Pasamos los tiquetes que recibimos del Runtime
        let signal = unsafe {
            let thread = &mut *thread_ptr;
            thread.execute_step(current_tickets)
        };

        println!("[Hilo {}] execute_step retornó: {:?}", tid, signal);

        // Convertir la señal del hilo en una respuesta para el Runtime
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
            // Las demás señales se pasan directamente
            ThreadSignal::Join(target_tid) => ThreadResponse::Join(target_tid),
            ThreadSignal::MutexLock(mutex_addr) => ThreadResponse::MutexLock(mutex_addr),
            ThreadSignal::MutexUnlock(mutex_addr) => ThreadResponse::MutexUnlock(mutex_addr),
        };

        let is_exit = matches!(response, ThreadResponse::Exit);
        let response_data = response.pack();

        // Devolvemos el control (y la respuesta) al Runtime
        transfer = unsafe { transfer.context.resume(response_data) };

        // --- El hilo se "congela" aquí hasta que el Runtime lo reanude ---

        // Cuando volvemos, el runtime nos ha despertado
        println!("[Hilo {}] despertado por el runtime", tid);

        if is_exit {
            eprintln!("[Hilo {}] ERROR: runtime despertó un hilo terminado", tid);
            std::process::abort();
        }

        // El Runtime nos envió un nuevo mensaje con el estado actualizado (incluyendo los tiquetes)
        // Desempacamos y actualizamos nuestros tiquetes por si cambiaron.
        if let TransferMessage::Init { current_tickets: new_tickets, .. } =
            unsafe { TransferMessage::unpack(transfer.data) }
        {
            current_tickets = new_tickets;
        } else {
            eprintln!("[Hilo {}] ERROR: esperaba mensaje Init al despertar", tid);
            std::process::abort();
        }
    }
}