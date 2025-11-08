//EL frontend que  llamaria la simulacion de la ciudad
use crate::api_context;
use crate::channels::SimpleMutex;
use crate::runtime::ThreadRuntimeV2;
use crate::signals::ThreadSignal;
use crate::thread::{SchedulerType, ThreadId, ThreadState};
use once_cell::sync::Lazy;
use std::sync::Mutex;

//Crear la instancia globak del runtime
static RUNTIME: Lazy<Mutex<ThreadRuntimeV2>> = Lazy::new(|| Mutex::new(ThreadRuntimeV2::new()));

//Parametros que se necesitan para la creacion de hilos (a futuro podria cambiar)
pub enum SchedulerParams {
    RoundRobin,
    Lottery { tickets: u32 },
    RealTime { deadline: u64 },
}

pub fn my_thread_create(
    name: &str,
    params: SchedulerParams,
    entry: Box<dyn FnMut(ThreadId) -> ThreadSignal + Send + 'static>,
) -> ThreadId {
    let mut runtime = RUNTIME.lock().unwrap();

    let (sched, tickets, deadline) = match params {
        SchedulerParams::RoundRobin => (SchedulerType::RoundRobin, 1, None),
        SchedulerParams::Lottery { tickets } => (SchedulerType::Lottery, tickets, None),
        SchedulerParams::RealTime { deadline } => (SchedulerType::RealTime, 0, Some(deadline)),
    };

    runtime.spawn(name, sched, entry, tickets, deadline)
}

//ceder el procesador por las buenas
pub fn my_thread_yield() {
    api_context::ctx_yield();
}

//matar al hilo actual
pub fn my_thread_end() {
    api_context::ctx_exit();
}

//Esperar a que un hilo especifico termine para poder continuar
pub fn my_thread_join(target_tid: ThreadId) -> ThreadSignal {
    ThreadSignal::Join(target_tid)
}
//Desvincula un hilo, haciendo que sus recursos se liberen al terminar
pub fn my_thread_detach(tid: ThreadId) {
    if let Ok(mut runtime) = RUNTIME.lock() {
        if let Some(thread) = runtime.threads.get_mut(&tid) {
            thread.detached = true;
            println!("[API] Hilo {} marcado como detached.", tid);
        }
    }
}

//Cambia ele shcedule que esta usando el hilo
pub fn my_thread_chsched(tid: ThreadId, paramns: SchedulerParams) {
    if let Ok(mut runtime) = RUNTIME.lock() {
        if let Some(thread) = runtime.threads.get_mut(&tid) {
            let (sched, tickets, deadline) = match paramns {
                SchedulerParams::RoundRobin => (SchedulerType::RoundRobin, 1, None),
                SchedulerParams::Lottery { tickets } => (SchedulerType::Lottery, tickets, None),
                SchedulerParams::RealTime { deadline } => {
                    (SchedulerType::RealTime, 0, Some(deadline))
                }
            };

            thread.sched_type = sched;
            thread.tickets = tickets;
            thread.deadline = deadline;
            println!("[API] Planificación del hilo {} actualizada.", tid);
        }
    }
}

pub struct MyMutex {
    internal: SimpleMutex,
}

//Inicia un mutex
pub fn my_mutex_init() -> MyMutex {
    MyMutex {
        internal: SimpleMutex::new(),
    }
}

//Bloquea el mutex, esperando si se ocupa 
pub fn my_mutex_lock(mutex: &MyMutex) -> ThreadSignal {
    let mutex_addr = &mutex.internal as *const _ as usize;
    ThreadSignal::MutexLock(mutex_addr)
}

//Liberar un mutex
pub fn my_mutex_unlock(mutex: &MyMutex) -> ThreadSignal{
    let mutex_addr = &mutex.internal as *const _ as usize;
    ThreadSignal::MutexUnlock(mutex_addr)
}

//Bloquear un utex sin esperar 
pub fn my_mutex_trylock(mutex: &MyMutex) -> bool {
    api_context::ctx_mutex_trylock(&mutex.internal)
}

//Destuye un mutex
pub fn my_mutex_destroy(_mutex: &mut MyMutex){
    //SE SUPONE QUE EN RUST, POR SU SISTEMA DE PROPIEDAD Y DROP, LA MEMORIA SE LIBERA AUTOMATICAMENTE
    //CUANDO LA VARIABLE SALE DEL SCOPE, POR LO QUE ACA NO SE HARIA NADA. 
    //A CHEQUEAR ESO INFO!!!!!!
}

pub fn run_simulation(cycles: usize) {
    RUNTIME.lock().unwrap().run(cycles);
}
