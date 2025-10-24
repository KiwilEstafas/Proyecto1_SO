//! nucleo de ejecucion con reloj logico colas y diccionario de hilos

use std::collections::{HashMap, VecDeque};
use crate::signals::ThreadSignal;
use crate::thread::{MyThread, SchedulerType, ThreadEntry, ThreadId, ThreadState};

pub struct ThreadRuntime {
    // reloj logico de la simulacion en milisegundos
    now_ms: u64,
    next_tid: ThreadId,
    pub threads: HashMap<ThreadId, MyThread>,
    pub ready: VecDeque<ThreadId>,
    pub current_tid: Option<ThreadId>,
}

impl ThreadRuntime {
    pub fn new() -> Self {
        Self {
            now_ms: 0,
            next_tid: 1,
            threads: HashMap::new(),
            ready: VecDeque::new(),
            current_tid: None,
        }
    }

    // avanza el reloj logico en dt_ms
    pub fn advance_time(&mut self, dt_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(dt_ms);
    }

    // devuelve el tiempo logico actual
    pub fn now(&self) -> u64 {
        self.now_ms
    }

    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        sched: SchedulerType,
        entry: ThreadEntry,
        tickets: Option<u32>,
        deadline: Option<u64>,
    ) -> ThreadId {
        let tid = self.next_tid;
        self.next_tid = self.next_tid.wrapping_add(1);

        let tickets = tickets.unwrap_or(1);
        let mut t = MyThread::new(tid, name.into(), sched, entry, tickets, deadline);
        t.state = ThreadState::Ready;

        self.threads.insert(tid, t);
        self.ready.push_back(tid);
        tid
    }

    // ejecuta un quantum logico
    pub fn run_once(&mut self) {
        // seleccionar hilo segun scheduler
        let Some(tid) = self.select_next_thread() else {
            return;
        };

        // marcar como running
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Running;
        }
        self.current_tid = Some(tid);

        // extraer temporalmente el entry para evitar doble prestamo mutable
        let mut entry = {
            let t = self.threads.get_mut(&tid).expect("thread debe existir");
            t.entry.take().expect("entry debe existir")
        };

        // ejecutar el paso del hilo sin tener prestado self threads
        let signal = (entry)(self, tid);

        // devolver el entry al hilo
        {
            let t = self.threads.get_mut(&tid).expect("thread debe existir");
            t.entry = Some(entry);
        }

        // manejar la senal
        match signal {
            ThreadSignal::Continue | ThreadSignal::Yield => {
                self.enqueue_ready(tid);
            }
            ThreadSignal::Block => {
                if let Some(t) = self.threads.get_mut(&tid) {
                    t.state = ThreadState::Blocked;
                }
            }
            ThreadSignal::Exit => {
                let (joiners_to_wake, detached) = {
                    let t = self.threads.get_mut(&tid).expect("thread debe existir");
                    t.state = ThreadState::Terminated;
                    (std::mem::take(&mut t.joiners), t.detached)
                };

                for jtid in joiners_to_wake {
                    self.wake(jtid);
                }

                if detached {
                    self.threads.remove(&tid);
                }
            }
        }

        self.current_tid = None;
    }

    // ejecuta multiples ciclos sin tocar el reloj
    pub fn run(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.run_once();
            if self.ready.is_empty() {
                break;
            }
        }
    }

    // selecciona el siguiente hilo a ejecutar segun su scheduler
    pub(crate) fn select_next_thread(&mut self) -> Option<ThreadId> {
        if self.ready.is_empty() {
            return None;
        }

        let front_tid = *self.ready.front().unwrap();
        let sched_type = self.threads.get(&front_tid)?.sched_type;

        match sched_type {
            SchedulerType::RoundRobin => self.schedule_roundrobin(),
            SchedulerType::Lottery => self.schedule_lottery(),
            SchedulerType::RealTime => self.schedule_realtime(),
        }
    }

    // mover hilo a ready
    pub(crate) fn enqueue_ready(&mut self, tid: ThreadId) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Ready;
        }
        self.ready.push_back(tid);
    }

    // despertar hilo bloqueado
    pub(crate) fn wake(&mut self, tid: ThreadId) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Ready;
        }
        self.ready.push_back(tid);
    }

    // obtener el tid del hilo actual si existe
    pub(crate) fn current(&self) -> Option<ThreadId> {
        self.current_tid
    }
}

