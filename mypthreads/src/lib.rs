//! Biblioteca de hilos preemptivos con cambio de contexto

// Módulos de la nueva arquitectura
pub mod runtime;
pub mod thread;      // Asumiendo que renombras thread_data2.rs a thread_v2.rs
pub mod thread_data;  
pub mod channels;      
pub mod api_context; 
pub mod signals;
pub mod context_wrapper;
pub mod mypthreads_api;

// Tipos públicos de la biblioteca
pub use runtime::ThreadRuntimeV2;
pub use thread::{MyThread, ContextThreadEntry, ThreadId, ThreadState, SchedulerType};
pub use channels::{ThreadChannels, JoinHandle, SimpleMutex, SharedData};
pub use api_context::*; // Exporta ctx_yield, ctx_exit, etc.
pub use signals::ThreadSignal; // Aún se usa en ContextThreadEntry
pub use context_wrapper::ThreadContext;
pub use thread_data::{TransferMessage, ThreadResponse}; // Útil para debugging