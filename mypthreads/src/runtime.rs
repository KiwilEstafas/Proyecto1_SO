use crate::channels::ThreadChannels;
use crate::context_wrapper::ThreadContext;
use crate::sched;
use crate::thread::{ContextThreadEntry, MyThread, SchedulerType, ThreadId, ThreadState};
use crate::thread_data::{ThreadResponse, TransferMessage};
use crate::SimpleMutex;
use std::collections::{HashMap, VecDeque};
use std::u64;


pub struct ThreadRuntimeV2 {
    now_ms: u64,
    next_tid: ThreadId,
    pub threads: HashMap<ThreadId, Box<MyThread>>,
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

        self.threads.insert(tid, Box::new(thread));
        self.ready.push_back(tid);

        //println!(
        //    "[Runtime] creado hilo {} (total: {})",
        //    tid,
        //    self.threads.len()
        //);

        tid
    }

    //pasar de un hilo bloqueado a listo
    pub fn unblock_thread(&mut self, tid: ThreadId) {
        if let Some(pos) = self.blocked.iter().position(|&id| id == tid) {
            let unblocked_tid = self.blocked.remove(pos);
            self.threads.get_mut(&unblocked_tid).unwrap().state = ThreadState::Ready;
            self.ready.push_back(unblocked_tid);
            //println!("[Runtime] Hilo {} desbloqueado.", unblocked_tid);
        }
    }

    //Decide que hilo ejecutar a continuacion
    fn select_next_thread(&mut self) -> Option<ThreadId> {
        let selected_tid = sched::select_next_thread(&self.ready, &self.threads, self.now_ms);

        if let Some(tid) = selected_tid {
            self.ready.retain(|&ready_tid| ready_tid != tid);
        }

        selected_tid
    }

    /// Mueve TODOS los hilos de la cola de bloqueados a la cola de listos.
    pub fn unblock_all_threads(&mut self) {
        // Tomamos todos los hilos bloqueados y los movemos a la cola de listos.
        for tid in self.blocked.drain(..) {
            if let Some(thread) = self.threads.get_mut(&tid) {
                thread.state = ThreadState::Ready;
                self.ready.push_back(tid);
                //println!(
                //    "[Runtime] Hilo {} desbloqueado por el ciclo de simulación.",
                //    tid
                //);
            }
        }
    }

    pub fn run_once(&mut self) {
        self.now_ms += 10;
        let Some(tid) = self.ready.pop_front() else {
            //println!("[Runtime] no hay hilos ready");
            return;
        };


        // obtener el hilo
        let thread = self.threads.get_mut(&tid).expect("hilo debe existir");
        let current_tickets = thread.tickets;
        thread.state = ThreadState::Running;

        // preparar mensaje inicial
        let thread_ptr = &mut **thread as *mut MyThread;
        let runtime_ctx_ptr = &mut self.runtime_context as *mut ThreadContext;

        let init_msg = TransferMessage::Init {
            thread_ptr,
            channels: self.channels.clone(),
            runtime_context_ptr: runtime_ctx_ptr as usize,
            current_tickets,
        };

        // hacer resume al hilo
        let response_data = unsafe { thread.context.resume_with_data(init_msg.pack()) };

        // procesar respuesta
        let response = unsafe { ThreadResponse::unpack(response_data) };

        //println!("[Runtime] hilo {} retornó: {:?}", tid, response);

        match response {
            ThreadResponse::Yield => {
                //println!("[Runtime] hilo {} hizo yield, reencolando", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Ready;
                self.ready.push_back(tid);
            }
            ThreadResponse::Block => {
                // println!("[Runtime] hilo {} se bloqueó", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Blocked;
                self.blocked.push(tid);
            }
            ThreadResponse::Exit => {
                //println!("[Runtime] hilo {} terminó", tid);
                let thread = self.threads.get_mut(&tid).unwrap();
                thread.state = ThreadState::Terminated;

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
                        //println!(
                        //    "[Runtime] Hilo {} no se bloquea, {} ya terminó.",
                        //    current_tid, target_tid
                        //);
                        self.ready.push_back(current_tid);
                    } else {
                        //println!("[Runtime] Hilo {} esperando a {}.", current_tid, target_tid);
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
                    //println!(
                    //    "[Runtime] Hilo {} se bloquea esperando un mutex.",
                    //    current_tid
                    //);
                    let thread = self.threads.get_mut(&current_tid).unwrap();
                    thread.state = ThreadState::Blocked;
                    self.blocked.push(current_tid);
                } else {
                    // El lock se adquirió, el hilo sigue listo.
                    //println!("[Runtime] Hilo {} adquirió un mutex.", current_tid);
                    self.ready.push_back(current_tid);
                }
            }
            ThreadResponse::MutexUnlock(mutex_addr) => {
                let mutex = unsafe { &*(mutex_addr as *const SimpleMutex) };
                let current_tid = tid;

                // `unlock` devuelve el siguiente hilo a despertar, si lo hay
                if let Some(unblocked_tid) = mutex.unlock(current_tid) {
                    //println!(
                    //    "[Runtime] Mutex liberado, despertando al hilo {}.",
                    //    unblocked_tid
                    //);
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
            self.run_once();

            if self.ready.is_empty() && self.blocked.is_empty() {
                //println!("[Runtime] no hay más hilos para ejecutar");
                break;
            }
        }
    }
}
