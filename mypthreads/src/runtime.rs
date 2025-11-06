//! runtime v2 que usa cambio de contexto real

use std::collections::{HashMap, VecDeque};
use crate::thread::{MyThread, ContextThreadEntry, ThreadId, ThreadState, SchedulerType};
use crate::context_wrapper::ThreadContext;
use crate::channels::ThreadChannels;
use crate::thread_data::{TransferMessage, ThreadResponse};

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
        
        let thread = MyThread::new(
            tid,
            name.into(),
            sched,
            tickets,
            deadline,
            entry,
        );
        
        self.threads.insert(tid, thread);
        self.ready.push_back(tid);
        
        println!("[Runtime] creado hilo {} (total: {})", tid, self.threads.len());
        
        tid
    }
    
    /// ejecuta un quantum - version con contextos
    pub fn run_once(&mut self) {
        let Some(tid) = self.ready.pop_front() else {
            println!("[Runtime] no hay hilos ready");
            return;
        };
        
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
        let response_data = unsafe {
            thread.context.resume_with_data(init_msg.pack())
        };
        
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
            }
            ThreadResponse::Continue => {
                self.ready.push_back(tid);
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
}
