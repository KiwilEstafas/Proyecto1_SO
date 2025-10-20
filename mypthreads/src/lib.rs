//! mypthreads – MVP cooperativo con Round Robin.
//! - ThreadRuntime: núcleo de hilos (cola READY, hilo actual, diccionario).
//! - MyThread: metadatos + entry (closure) que devuelve una señal.
//! - Señales: Continue | Yield | Exit
//! - Wrappers my_thread_create / my_thread_yield / my_thread_end

use std::collections::{HashMap, VecDeque};

pub type ThreadId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerType {
    RoundRobin,
    Lottery,
    RealTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadSignal {
    Continue,
    Yield,
    Exit,
}

pub type ThreadEntry =
    Box<dyn FnMut(&mut ThreadRuntime, ThreadId) -> ThreadSignal + Send + 'static>;

pub struct MyThread {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    // Usamos Option para poder 'take()' el closure temporalmente sin doble préstamo.
    entry: Option<ThreadEntry>,
}

impl MyThread {
    fn new(id: ThreadId, name: String, sched_type: SchedulerType, entry: ThreadEntry) -> Self {
        Self {
            id,
            name,
            state: ThreadState::New,
            sched_type,
            entry: Some(entry),
        }
    }
}

pub struct ThreadRuntime {
    next_tid: ThreadId,
    pub threads: HashMap<ThreadId, MyThread>,
    pub ready: VecDeque<ThreadId>,
    pub current_tid: Option<ThreadId>,
}

impl ThreadRuntime {
    pub fn new() -> Self {
        Self {
            next_tid: 1,
            threads: HashMap::new(),
            ready: VecDeque::new(),
            current_tid: None,
        }
    }

    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        sched: SchedulerType,
        entry: ThreadEntry,
    ) -> ThreadId {
        let tid = self.next_tid;
        self.next_tid += 1;

        let mut t = MyThread::new(tid, name.into(), sched, entry);
        t.state = ThreadState::Ready;

        self.threads.insert(tid, t);
        self.ready.push_back(tid);
        tid
    }

    /// Ejecuta un "quantum" lógico con Round Robin.
    pub fn run_once(&mut self) {
        let Some(tid) = self.ready.pop_front() else {
            return;
        };

        // Marcar como RUNNING
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Running;
        }
        self.current_tid = Some(tid);

        // === Punto clave: extraer temporalmente el 'entry' para evitar préstamo doble ===
        let mut entry = {
            let t = self.threads.get_mut(&tid).expect("thread debe existir");
            t.entry.take().expect("entry debe existir")
        };

        // Ejecutar el paso del hilo SIN tener prestado self.threads
        let signal = (entry)(self, tid);

        // Devolver el entry al hilo para futuras llamadas
        {
            let t = self.threads.get_mut(&tid).expect("thread debe existir");
            t.entry = Some(entry);
        }
        // === fin del manejo de entry ===

        // Manejar la señal
        match signal {
            ThreadSignal::Continue => {
                self.enqueue_ready(tid);
            }
            ThreadSignal::Yield => {
                self.enqueue_ready(tid);
            }
            ThreadSignal::Exit => {
                if let Some(t) = self.threads.get_mut(&tid) {
                    t.state = ThreadState::Terminated;
                }
            }
        }

        self.current_tid = None;
    }

    fn enqueue_ready(&mut self, tid: ThreadId) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Ready;
        }
        self.ready.push_back(tid);
    }
}

/* =========
 * Wrappers estilo mypthreads (MVP)
 * ========= */

pub fn my_thread_create(
    rt: &mut ThreadRuntime,
    name: &str,
    sched: SchedulerType,
    entry: ThreadEntry,
) -> ThreadId {
    rt.spawn(name, sched, entry)
}

pub fn my_thread_end() -> ThreadSignal {
    ThreadSignal::Exit
}

pub fn my_thread_yield() -> ThreadSignal {
    ThreadSignal::Yield
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rr_interleaves_two_threads() {
        let mut rt = ThreadRuntime::new();

        let mut a_count = 0;
        let mut b_count = 0;

        let a = my_thread_create(
            &mut rt,
            "A",
            SchedulerType::RoundRobin,
            Box::new(move |_rt, _tid| {
                a_count += 1;
                if a_count >= 3 {
                    return my_thread_end();
                }
                my_thread_yield()
            }),
        );

        let _b = my_thread_create(
            &mut rt,
            "B",
            SchedulerType::RoundRobin,
            Box::new(move |_rt, _tid| {
                b_count += 1;
                if b_count >= 2 {
                    return my_thread_end();
                }
                my_thread_yield()
            }),
        );

        for _ in 0..10 {
            rt.run_once();
        }

        assert_eq!(rt.threads.get(&a).unwrap().state, ThreadState::Terminated);
    }
}
