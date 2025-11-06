//! estructuras para pasar datos entre contextos via Transfer.data

use crate::thread::{ThreadId, MyThread};
use crate::channels::ThreadChannels;

/// tipo de mensaje que se pasa via Transfer.data
#[repr(C)]
pub enum TransferMessage {
    /// mensaje inicial: contiene puntero al hilo, canales Y runtime context
    Init {
        thread_ptr: *mut MyThread,
        channels: ThreadChannels,
        runtime_context_ptr: usize,
    },
    /// mensaje de continuacion: el runtime dice "continua ejecutando"
    Continue,
    /// mensaje de terminacion: el runtime dice "ya terminaste, limpia"
    Terminate,
}

impl TransferMessage {
    /// empaqueta el mensaje como usize para Transfer.data
    pub fn pack(self) -> usize {
        let boxed = Box::new(self);
        Box::into_raw(boxed) as usize
    }
    
    /// desempaqueta el mensaje desde Transfer.data
    /// SAFETY: data debe ser un puntero valido creado por pack()
    pub unsafe fn unpack(data: usize) -> Self {
        if data == 0 {
            return TransferMessage::Continue;
        }
        let boxed = Box::from_raw(data as *mut TransferMessage);
        *boxed
    }
}

/// mensaje de respuesta del hilo al runtime
#[repr(C)]
#[derive(Debug)]
pub enum ThreadResponse {
    Yield,
    Block,
    Exit,
    Continue,
}

impl ThreadResponse {
    pub fn pack(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }
    
    pub unsafe fn unpack(data: usize) -> Self {
        if data == 0 {
            return ThreadResponse::Continue;
        }
        let boxed = Box::from_raw(data as *mut ThreadResponse);
        *boxed
    }
}

/// contexto global del hilo almacenado en thread-local
pub struct ThreadGlobalContext {
    pub tid: ThreadId,
    pub channels: ThreadChannels,
    // NO guardamos punteros aquí - dejamos que el wrapper maneje todo
}

// Thread-local storage para el contexto del hilo
thread_local! {
    static GLOBAL_CTX: std::cell::RefCell<Option<ThreadGlobalContext>> = 
        std::cell::RefCell::new(None);
}

impl ThreadGlobalContext {
    /// inicializa el contexto global del hilo actual
    pub fn init(tid: ThreadId, channels: ThreadChannels) {
        GLOBAL_CTX.with(|ctx| {
            *ctx.borrow_mut() = Some(ThreadGlobalContext {
                tid,
                channels,
            });
        });
    }
    
    /// obtiene una referencia al contexto global
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&ThreadGlobalContext) -> R,
    {
        GLOBAL_CTX.with(|ctx| {
            let ctx_ref = ctx.borrow();
            let ctx = ctx_ref.as_ref().expect("thread context no inicializado");
            f(ctx)
        })
    }
}
