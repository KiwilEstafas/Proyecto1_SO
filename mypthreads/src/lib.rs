//! mypthreads – MVP cooperativo con Round Robin.
//! - ThreadRuntime: núcleo de hilos (cola READY, hilo actual, diccionario).
//! - MyThread: metadatos + entry (closure) que devuelve una señal.
//! - Señales: Continue | Yield | Exit
//! - Wrappers my_thread_create / my_thread_yield / my_thread_end

use std::collections::{HashMap, VecDeque};
use rand::Rng; 

pub type ThreadId = u32;

//Estados del hilo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
    Detached,
}

//Tipos de planificador a implementar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerType {
    RoundRobin,
    Lottery,
    RealTime,
}

//Señales del hilo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadSignal {
    Continue,
    Yield,
    Exit,
}

pub type ThreadEntry =
    Box<dyn FnMut(&mut ThreadRuntime, ThreadId) -> ThreadSignal + Send + 'static>;

//Estructura del hilo
pub struct MyThread {
    pub id: ThreadId,
    pub name: String,
    pub state: ThreadState,
    pub sched_type: SchedulerType,
    pub tickets: u32, //Para el lottery
    // Usamos Option para poder 'take()' el closure temporalmente sin doble prestamo.
    entry: Option<ThreadEntry>,
}

impl MyThread {
    fn new(id: ThreadId, name: String, sched_type: SchedulerType, entry: ThreadEntry, tickets: u32 ) -> Self {
        Self {
            id,
            name,
            state: ThreadState::New,
            sched_type,
            tickets,
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
        tickets: Option<u32> //Aceptar ticketes opcionales
    ) -> ThreadId {
        let tid = self.next_tid;
        self.next_tid += 1;

        let tickets = tickets.unwrap_or(1);
        let mut t = MyThread::new(tid, name.into(), sched, entry, tickets);
        t.state = ThreadState::Ready;

        self.threads.insert(tid, t);
        self.ready.push_back(tid);
        tid
    }

    /// Ejecuta un "quantum" lógico con Round Robin.
    pub fn run_once(&mut self) {
        // Seleccionar hilo según scheduler
        let Some(tid) = self.select_next_thread() else {
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

    //para que se ejecute continueamente
    pub fn run(&mut self, cycles: usize){
        for _ in 0..cycles{
            self.run_once();
            if self.ready.is_empty(){
                break;
            }
        }
    }

    //Selecciona el siguiente hilo a ejecutar segun su scheduler
    fn select_next_thread(&mut self) -> Option<ThreadId>{
        if self.ready.is_empty(){
            return None;
        }

        //Fijarse en el primer hilo para ver cual es
        let front_tid = *self.ready.front().unwrap();
        let sched_type = self.threads.get(&front_tid)?.sched_type;

        match sched_type {
            SchedulerType::RoundRobin => self.schedule_roundrobin(),
            SchedulerType::Lottery => self.schedule_lottery(),
            SchedulerType::RealTime => self.schedule_realtime(),
        }
    }

    fn schedule_roundrobin(&mut self) -> Option<ThreadId>{
        self.ready.pop_front()
    }

    fn schedule_lottery(&mut self) -> Option<ThreadId> {
        let ready_threads: Vec<&MyThread> = self.ready.iter()
            .filter_map(|tid| self.threads.get(tid))
            .collect();

        if ready_threads.is_empty() {
            return None;
        }

        // Sumar tickets
        let total_tickets: u32 = ready_threads.iter().map(|t| t.tickets).sum();
        let mut rng = rand::rng();
        let mut pick = rng.random_range(0..total_tickets);

        // Buscar ganador
        for t in &ready_threads {
            if pick < t.tickets {
                // Remover el thread de la cola ready
                self.ready.retain(|&tid| tid != t.id);
                return Some(t.id);
            }
            pick -= t.tickets;
        }

        // fallback, no debería pasar
        let t = ready_threads[0];
        self.ready.retain(|&tid| tid != t.id);
        Some(t.id)
    }

    fn schedule_realtime(&mut self) -> Option<ThreadId> {
        //TODO
        println!("Sin implementar por el momento!!, mientras usa RR");
        self.schedule_roundrobin()
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
    tickets: Option<u32>,
) -> ThreadId {
    rt.spawn(name, sched, entry, tickets)
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
    use std::sync::{Arc, Mutex};

    #[test]
    fn rr_interleaves_two_threads() {
        let mut rt = ThreadRuntime::new();

        let a_count = Arc::new(Mutex::new(0));
        let b_count = Arc::new(Mutex::new(0));

        // Hilo A
        let a_count_clone = Arc::clone(&a_count);
        let a = my_thread_create(
            &mut rt,
            "A",
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
        );

        // Hilo B
        let b_count_clone = Arc::clone(&b_count);
        let _b = my_thread_create(
            &mut rt,
            "B",
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
        );

        rt.run(10);

        // Comprobamos que los hilos terminaron
        assert_eq!(rt.threads.get(&a).unwrap().state, ThreadState::Terminated);
        assert_eq!(*a_count.lock().unwrap(), 3);
        assert_eq!(*b_count.lock().unwrap(), 2);
    }

    #[test]
    fn lottery_scheduler_biases_high_ticket_threads() {
        let mut rt = ThreadRuntime::new();

        let a_count = Arc::new(Mutex::new(0));
        let b_count = Arc::new(Mutex::new(0));

        // Hilo A con 3 tickets
        let a_count_clone = Arc::clone(&a_count);
        let _a = rt.spawn(
            "A",
            SchedulerType::Lottery,
            Box::new(move |_rt, _tid| {
                let mut cnt = a_count_clone.lock().unwrap();
                *cnt += 1;
                my_thread_yield()
            }),
            Some(3),
        );

        // Hilo B con 1 ticket
        let b_count_clone = Arc::clone(&b_count);
        let _b = rt.spawn(
            "B",
            SchedulerType::Lottery,
            Box::new(move |_rt, _tid| {
                let mut cnt = b_count_clone.lock().unwrap();
                *cnt += 1;
                my_thread_yield()
            }),
            Some(1),
        );

        // Ejecutar muchos ciclos
        rt.run(1000);

        let a_total = *a_count.lock().unwrap();
        let b_total = *b_count.lock().unwrap();

        println!("A ran {} times, B ran {} times", a_total, b_total);

        assert!(a_total > b_total);
    }
}