//! mypthreads mvp cooperativo con round robin
//! threadruntime es el nucleo de hilos con cola ready hilo actual y diccionario
//! mythread almacena metadatos y la entry que devuelve una senal
//! senales continue yield block exit
//! wrappers my_thread_create my_thread_yield my_thread_end my_thread_join my_thread_detach
//! mutex minimo con init y destroy

use std::collections::{HashMap, VecDeque};
use rand::Rng;

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

// senales del hilo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadSignal {
    Continue, // hilo sigue y se reencola como yield en este mvp
    Yield,    // hilo cede y se reencola al final
    Block,    // hilo se bloquea y no se reencola
    Exit,     // hilo termina
}

// la funcion de entrada del hilo recibe el runtime y su tid y devuelve una senal
pub type ThreadEntry =
    Box<dyn FnMut(&mut ThreadRuntime, ThreadId) -> ThreadSignal + Send + 'static>;

// estructura del hilo
pub struct MyThread {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    pub tickets: u32,             // para lottery
    pub deadline: Option<u64>,    // deadline absoluto en milisegundos logicos
    pub detached: bool,           // si es true no se puede hacer join y se limpia al terminar
    pub joiners: Vec<ThreadId>,   // hilos que esperan a este hilo
    // usamos option para poder take del closure temporalmente y evitar doble prestamo mutable
    entry: Option<ThreadEntry>,
}

impl MyThread {
    fn new(
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
            entry: Some(entry),
        }
    }
}

// nucleo de ejecucion con cola ready y reloj logico
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
            ThreadSignal::Continue => {
                self.enqueue_ready(tid);
            }
            ThreadSignal::Yield => {
                self.enqueue_ready(tid);
            }
            ThreadSignal::Block => {
                // el hilo queda en estado blocked y no se reencola
                if let Some(t) = self.threads.get_mut(&tid) {
                    t.state = ThreadState::Blocked;
                }
            }
            ThreadSignal::Exit => {
                // obtener joiners y bandera detached sin asignaciones previas
                let (joiners_to_wake, detached) = {
                    let t = self.threads.get_mut(&tid).expect("thread debe existir");
                    t.state = ThreadState::Terminated;
                    (std::mem::take(&mut t.joiners), t.detached)
                };

                // despertar joiners
                for jtid in joiners_to_wake {
                    self.wake(jtid);
                }

                // si es detached se limpia la estructura
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
    fn select_next_thread(&mut self) -> Option<ThreadId> {
        if self.ready.is_empty() {
            return None;
        }

        // mirar el primer hilo para decidir la politica a aplicar
        let front_tid = *self.ready.front().unwrap();
        let sched_type = self.threads.get(&front_tid)?.sched_type;

        match sched_type {
            SchedulerType::RoundRobin => self.schedule_roundrobin(),
            SchedulerType::Lottery => self.schedule_lottery(),
            SchedulerType::RealTime => self.schedule_realtime(),
        }
    }

    // politica round robin
    fn schedule_roundrobin(&mut self) -> Option<ThreadId> {
        self.ready.pop_front()
    }

    // politica lottery con tickets
    fn schedule_lottery(&mut self) -> Option<ThreadId> {
        let ready_threads: Vec<&MyThread> = self
            .ready
            .iter()
            .filter_map(|tid| self.threads.get(tid))
            .collect();

        if ready_threads.is_empty() {
            return None;
        }

        // sumar tickets
        let total_tickets: u32 = ready_threads.iter().map(|t| t.tickets).sum();

        // sorteo usando rand 0 dot 9
        let mut rng = rand::rng();
        let mut pick: u32 = rng.random_range(0..total_tickets);

        // buscar ganador
        for t in &ready_threads {
            if pick < t.tickets {
                // remover el ganador de la cola ready
                self.ready.retain(|&tid| tid != t.id);
                return Some(t.id);
            }
            pick -= t.tickets;
        }

        // caso de respaldo
        let t = ready_threads[0];
        self.ready.retain(|&tid| tid != t.id);
        Some(t.id)
    }

    // politica tiempo real usando edf sobre reloj logico
    // se elige el hilo con menor tiempo restante a su deadline absoluto
    // si nadie tiene deadline se cae a round robin
    fn schedule_realtime(&mut self) -> Option<ThreadId> {
        let now = self.now_ms;

        let ready_threads: Vec<&MyThread> = self
            .ready
            .iter()
            .filter_map(|tid| self.threads.get(tid))
            .collect();

        if ready_threads.is_empty() {
            return None;
        }

        // calcular tiempo restante a deadline y elegir el minimo
        let candidate = ready_threads
            .iter()
            .filter_map(|t| t.deadline.map(|d| (t.id, d.saturating_sub(now))))
            .min_by_key(|&(_, remaining)| remaining)
            .map(|(id, _)| id);

        if let Some(tid) = candidate {
            self.ready.retain(|&id| id != tid);
            Some(tid)
        } else {
            self.schedule_roundrobin()
        }
    }

    // mover hilo a ready
    fn enqueue_ready(&mut self, tid: ThreadId) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Ready;
        }
        self.ready.push_back(tid);
    }

    // despertar hilo bloqueado
    fn wake(&mut self, tid: ThreadId) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.state = ThreadState::Ready;
        }
        self.ready.push_back(tid);
    }

    // obtener el tid del hilo actual si existe
    fn current(&self) -> Option<ThreadId> {
        self.current_tid
    }
}

/* =========
 * wrappers estilo mypthreads mvp
 * ========= */

pub fn my_thread_create(
    rt: &mut ThreadRuntime,
    name: &str,
    sched: SchedulerType,
    entry: ThreadEntry,
    tickets: Option<u32>,
    deadline: Option<u64>,
) -> ThreadId {
    rt.spawn(name, sched, entry, tickets, deadline)
}

pub fn my_thread_end() -> ThreadSignal {
    ThreadSignal::Exit
}

pub fn my_thread_yield() -> ThreadSignal {
    ThreadSignal::Yield
}

// bloquea al hilo actual hasta que el objetivo termine
// si el objetivo ya termino devuelve yield para que el llamante continue
// si el objetivo esta detached devuelve yield y no se bloquea
// si el objetivo no existe devuelve yield
pub fn my_thread_join(rt: &mut ThreadRuntime, target: ThreadId) -> ThreadSignal {
    // obtener hilo actual
    let Some(self_tid) = rt.current() else {
        // si no hay hilo actual no podemos bloquear
        return ThreadSignal::Yield;
    };

    // leer estado del objetivo
    let mut should_block = false;
    let mut can_join = true;

    if let Some(t) = rt.threads.get(&target) {
        if t.detached {
            can_join = false;
        } else if t.state != ThreadState::Terminated {
            should_block = true;
        }
    } else {
        can_join = false;
    }

    if !can_join {
        return ThreadSignal::Yield;
    }

    if should_block {
        // registrar al actual como joiner del objetivo
        if let Some(t) = rt.threads.get_mut(&target) {
            t.joiners.push(self_tid);
        }
        // senal para bloquear al actual
        return ThreadSignal::Block;
    }

    // si ya esta terminado no bloqueamos
    ThreadSignal::Yield
}

// marca un hilo como detached
// si ya termino se limpia su control block
pub fn my_thread_detach(rt: &mut ThreadRuntime, target: ThreadId) {
    if let Some(t) = rt.threads.get_mut(&target) {
        t.detached = true;
        if t.state == ThreadState::Terminated {
            // limpiar inmediatamente si ya termino
            rt.threads.remove(&target);
        }
    }
}

/* =========
 * mutex minimo con init y destroy
 * ========= */

// estructura de mutex pensada para extender con lock y unlock
pub struct MyMutex {
    initialized: bool,
    locked: bool,
    _owner: Option<ThreadId>,       // guion bajo para indicar que aun no se usa
    wait_queue: VecDeque<ThreadId>,
}

impl MyMutex {
    // inicializa el mutex
    pub fn my_mutex_init() -> Self {
        Self {
            initialized: true,
            locked: false,
            _owner: None,
            wait_queue: VecDeque::new(),
        }
    }

    // destruye el mutex
    // devuelve 0 si exito y 1 si no se puede destruir
    // no se permite destruir si esta bloqueado o si no estaba inicializado
    pub fn my_mutex_destroy(&mut self) -> i32 {
        if !self.initialized {
            return 1;
        }
        if self.locked {
            return 1;
        }
        if !self.wait_queue.is_empty() {
            return 1;
        }
        self.initialized = false;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn rr_interleaves_two_threads() {
        let mut rt = ThreadRuntime::new();

        let a_count = Arc::new(Mutex::new(0));
        let b_count = Arc::new(Mutex::new(0));

        // hilo a
        let a_count_clone = Arc::clone(&a_count);
        let a = my_thread_create(
            &mut rt,
            "a",
            SchedulerType::RoundRobin,
            Box::new(move |_rt, _tid| {
                let mut cnt = a_count_clone.lock().unwrap();
                *cnt += 1;
                if *cnt >= 3 {
                    return my_thread_end();
                }
                my_thread_yield()
            }),
            None,
            None,
        );

        // hilo b
        let b_count_clone = Arc::clone(&b_count);
        let _b = my_thread_create(
            &mut rt,
            "b",
            SchedulerType::RoundRobin,
            Box::new(move |_rt, _tid| {
                let mut cnt = b_count_clone.lock().unwrap();
                *cnt += 1;
                if *cnt >= 2 {
                    return my_thread_end();
                }
                my_thread_yield()
            }),
            None,
            None,
        );

        rt.run(10);

        // verificacion basica
        assert_eq!(rt.threads.get(&a).unwrap().state, ThreadState::Terminated);
        assert_eq!(*a_count.lock().unwrap(), 3);
        assert_eq!(*b_count.lock().unwrap(), 2);
    }

    #[test]
    fn lottery_scheduler_biases_high_ticket_threads() {
        let mut rt = ThreadRuntime::new();

        let a_count = Arc::new(Mutex::new(0));
        let b_count = Arc::new(Mutex::new(0));

        // hilo a con 3 tickets
        let a_count_clone = Arc::clone(&a_count);
        let _a = rt.spawn(
            "a",
            SchedulerType::Lottery,
            Box::new(move |_rt, _tid| {
                let mut cnt = a_count_clone.lock().unwrap();
                *cnt += 1;
                my_thread_yield()
            }),
            Some(3),
            None,
        );

        // hilo b con 1 ticket
        let b_count_clone = Arc::clone(&b_count);
        let _b = rt.spawn(
            "b",
            SchedulerType::Lottery,
            Box::new(move |_rt, _tid| {
                let mut cnt = b_count_clone.lock().unwrap();
                *cnt += 1;
                my_thread_yield()
            }),
            Some(1),
            None,
        );

        // ejecutar muchos ciclos
        rt.run(1000);

        let a_total = *a_count.lock().unwrap();
        let b_total = *b_count.lock().unwrap();

        println!("a ran {} times, b ran {}", a_total, b_total);

        assert!(a_total > b_total);
    }

    #[test]
    fn realtime_scheduler_runs_earliest_deadline_first_with_clock() {
        let mut rt = ThreadRuntime::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        // hilo con deadline mas lejano
        let order_low = Arc::clone(&order);
        let entry_low: ThreadEntry = Box::new(move |_rt, _tid| {
            order_low.lock().unwrap().push("low".to_string());
            my_thread_end()
        });

        // hilo con deadline mas cercano
        let order_high = Arc::clone(&order);
        let entry_high: ThreadEntry = Box::new(move |_rt, _tid| {
            order_high.lock().unwrap().push("high".to_string());
            my_thread_end()
        });

        my_thread_create(&mut rt, "low", SchedulerType::RealTime, entry_low, None, Some(50));
        my_thread_create(&mut rt, "high", SchedulerType::RealTime, entry_high, None, Some(10));

        rt.advance_time(0);
        rt.run_once();

        let seq = order.lock().unwrap().clone();
        assert_eq!(seq[0], "high");
    }

    #[test]
    fn mutex_init_destroy() {
        let mut m = MyMutex::my_mutex_init();
        assert_eq!(m.my_mutex_destroy(), 0);
    }
}

