//! mypthreads mvp cooperativo
//! expone api rust y api ffi con firmas tipo pthreads

pub mod runtime;
pub mod thread;
pub mod thread_v2;    
pub mod channels;      
pub mod api_context; 
pub mod sched;
pub mod signals;
pub mod api_rust;
pub mod mutex;
pub mod context_wrapper;

// exportar tipos v2
pub use thread_v2::{MyThreadV2, ContextThreadEntry};
pub use channels::{ThreadChannels, JoinHandle, SimpleMutex, SharedData};
pub use api_context::*;

pub use runtime::ThreadRuntime;
pub use thread::{MyThread, ThreadId, ThreadState, SchedulerType, ThreadEntry};
pub use signals::ThreadSignal;
pub use api_rust::{
    my_thread_create, my_thread_end, my_thread_yield,
    my_thread_join, my_thread_detach, my_thread_chsched, 
    my_mutex_lock, my_mutex_unlock, my_mutex_trylock
};
pub use mutex::MyMutex;
pub use context_wrapper::ThreadContext;

