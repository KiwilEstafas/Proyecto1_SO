//! runtime v2 que usa cambio de contexto real

use crate::SimpleMutex;
use crate::channels::ThreadChannels;
use crate::context_wrapper::ThreadContext;
use crate::thread::{ContextThreadEntry, MyThread, SchedulerType, ThreadId, ThreadState};
use crate::thread_data::{ThreadResponse, TransferMessage};
use std::collections::{HashMap, VecDeque};
use crate::sched;

pub struct ThreadRuntimeV2 {
    now_ms: u64,
    next_tid: ThreadId,
    pub threads: HashMap<ThreadId, MyThread>,
    pub ready: VecDeque<ThreadId>,
    pub blocked: Vec<ThreadId>,
    pub runtime_context: ThreadContext,
    pub channels: ThreadChannels,
}

impl ThreadRuntimeV2 {
    pub fn new() -> Self {
        Self {
            now_ms: 0,
            next_tid: 1,
            threads: HashMap::new(),
            ready: VecDeque::new(),
            blocked: Vec::new(),
            runtime_context: ThreadContext::new_runtime(),
            channels: ThreadChannels::new(),
        }
    }

    /// crea un nuevo hilo v2
    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        sched: SchedulerType,
        entry: ContextThreadEntry,
        tickets: u32,
        deadline: Option<u64>,
    ) -> ThreadId {
        let tid = self.next_tid;
        self.next_tid += 1;

        let thread = MyThread::new(tid, name.into(), sched, tickets, deadline, entry);

        self.threads.insert(tid, thread);
        self.ready.push_back(tid);

        println!(
            "[Runtime] creado hilo {} (total: {})",
            tid,
            self.threads.len()
        );

        tid
    }

    //pasar de un hilo bloqueado a listo
    pub fn unblock_thread(&mut self, tid: ThreadId) {
        if let Some(pos) = self.blocked.iter().position(|&id| id == tid) {
            let unblocked_tid = self.blocked.remove(pos);
            self.threads.get_mut(&unblocked_tid).unwrap().state = ThreadState::Ready;
            self.ready.push_back(unblocked_tid);
            println!("[Runtime] Hilo {} desbloqueado.", unblocked_tid);
        }
    }

    /// ejecuta un quantum - version con contextos
    pub fn run_once(&mut self) {
        self.now_ms += 10;
        
        // Usar el scheduler para seleccionar el siguiente hilo
        let Some(tid) = sched::select_next_thread(&self.ready, &self.threads, self.now_ms) else {
            println!("[Runtime] no hay hilos ready");
            return;
        };

        // Remover el hilo seleccionado de la cola de ready
        self.ready.retain(|&ready_tid| ready_tid != tid);

        println!("[Runtime] seleccionado hilo {} para ejecutar", tid);

        // obtener el hilo
        let thread = self.threads.get_mut(&tid).expect("hilo debe existir");
        thread.state = ThreadState::Running;

        // preparar mensaje inicial
        let thread_ptr = thread as *mut MyThread;
        let runtime_ctx_ptr = &mut self.runtime_context as *mut ThreadContext;

        let init_msg = TransferMessage::Init {
            thread_ptr,
            channels: self.channels.clone(),
            runtime_context_ptr: runtime_ctx_ptr as usize,
        };

        // hacer resume al hilo
        let response_data = unsafe { thread.context.resume_with_data(init_msg.pack()) };

        // procesar respuesta
        let response = unsafe { ThreadResponse::unpack(response_data) };

        println!("[Runtime] hilo {} retornó: {:?}", tid, response);

        match response {
            ThreadResponse::Yield => {
                println!("[Runtime] hilo {} hizo yield, reencolando", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Ready;
                self.ready.push_back(tid);
            }
            ThreadResponse::Block => {
                println!("[Runtime] hilo {} se bloqueó", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Blocked;
                self.blocked.push(tid);
            }
            ThreadResponse::Exit => {
                println!("[Runtime] hilo {} terminó", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Terminated;

                //MOdificacion papra el join
                //Despierta TODOS los hilos que estaban esperando por este en cuestion
                let joiners_unblock = thread.joiners.clone(); //Es mejor clonar para evitar problemas de borrow
                for joiner_tid in joiners_unblock {
                    self.unblock_thread(joiner_tid);
                }
            }
            ThreadResponse::Continue => {
                self.ready.push_back(tid);
            }
            ThreadResponse::Join(target_tid) => {
                let current_tid = tid;
                let mut should_block = true;

                if let Some(target) = self.threads.get(&target_tid) {
                    if target.state == ThreadState::Terminated {
                        should_block = false;
                        println!(
                            "[Runtime] Hilo {} no se bloquea, {} ya terminó.",
                            current_tid, target_tid
                        );
                        self.ready.push_back(current_tid);
                    } else {
                        println!("[Runtime] Hilo {} esperando a {}.", current_tid, target_tid);
                        self.threads
                            .get_mut(&target_tid)
                            .unwrap()
                            .joiners
                            .push(current_tid);
                    }
                } else {
                    should_block = false;
                    self.ready.push_back(current_tid);
                }

                if should_block {
                    let thread = self.threads.get_mut(&current_tid).unwrap();
                    thread.state = ThreadState::Blocked;
                    self.blocked.push(current_tid);
                }
            }
            ThreadResponse::MutexLock(mutex_addr) => {
                let mutex = unsafe { &*(mutex_addr as *const SimpleMutex) };
                let current_tid = tid;

                // `lock` ahora devuelve `true` si se debe bloquear
                if mutex.lock(current_tid) {
                    // El lock no se pudo adquirir, bloquear el hilo.
                    println!(
                        "[Runtime] Hilo {} se bloquea esperando un mutex.",
                        current_tid
                    );
                    let thread = self.threads.get_mut(&current_tid).unwrap();
                    thread.state = ThreadState::Blocked;
                    self.blocked.push(current_tid);
                } else {
                    // El lock se adquirió, el hilo sigue listo.
                    println!("[Runtime] Hilo {} adquirió un mutex.", current_tid);
                    self.ready.push_back(current_tid);
                }
            }
            ThreadResponse::MutexUnlock(mutex_addr) => {
                let mutex = unsafe { &*(mutex_addr as *const SimpleMutex) };
                let current_tid = tid;

                // `unlock` devuelve el siguiente hilo a despertar, si lo hay
                if let Some(unblocked_tid) = mutex.unlock(current_tid) {
                    println!(
                        "[Runtime] Mutex liberado, despertando al hilo {}.",
                        unblocked_tid
                    );
                    self.unblock_thread(unblocked_tid);
                }

                // El hilo que liberó el mutex vuelve a estar listo.
                self.ready.push_back(current_tid);
            }
        }
    }

    /// ejecuta multiples ciclos
    pub fn run(&mut self, cycles: usize) {
        for i in 0..cycles {
            println!("\n[Runtime] === Ciclo {} ===", i + 1);
            self.run_once();

            if self.ready.is_empty() && self.blocked.is_empty() {
                println!("[Runtime] no hay más hilos para ejecutar");
                break;
            }
        }
    }
    
    pub fn current_time(&self) -> u64 {
        self.now_ms
    }
}
