//! mypthreads mvp cooperativo
//! expone api rust y api ffi con firmas tipo pthreads

pub mod runtime;
pub mod thread;
pub mod sched;
pub mod signals;
pub mod api_rust;
pub mod mutex;

pub use runtime::ThreadRuntime;
pub use thread::{MyThread, ThreadId, ThreadState, SchedulerType, ThreadEntry};
pub use signals::ThreadSignal;
pub use api_rust::{
    my_thread_create, my_thread_end, my_thread_yield,
    my_thread_join, my_thread_detach, my_thread_chsched, 
    my_mutex_lock, my_mutex_unlock, my_mutex_trylock
};
pub use mutex::MyMutex;

