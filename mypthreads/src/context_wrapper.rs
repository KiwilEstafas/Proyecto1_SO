// wrapper minimalista sobre el crate context

use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;

const STACK_SIZE: usize = 8192; // 8kb por hilo

pub struct ThreadContext {
    pub context: Option<Context>, // CAMBIADO a pub para acceso directo
    _stack: Box<ProtectedFixedSizeStack>,
}

impl ThreadContext {
    pub fn new(entry: extern "C" fn(Transfer) -> !) -> Self {
        let stack = ProtectedFixedSizeStack::new(STACK_SIZE)
            .expect("no se pudo crear la pila");
        
        let context = unsafe {
            Context::new(&stack, entry)
        };
        
        Self {
            context: Some(context),
            _stack: Box::new(stack),
        }
    }

    /// resume con data
    pub unsafe fn resume_with_data(&mut self, data: usize) -> usize {
        let ctx = self.context.take()
            .expect("context debe existir");
        
        let transfer = ctx.resume(data);
        
        // guardar el contexto que nos devolvió
        self.context = Some(transfer.context);
        
        transfer.data
    }

    pub fn new_runtime() -> Self {
        let stack = ProtectedFixedSizeStack::new(STACK_SIZE)
            .expect("no se pudo crear la pila del runtime");
        
        Self {
            context: None,
            _stack: Box::new(stack),
        }
    }
}

impl std::fmt::Debug for ThreadContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadContext")
            .field("has_context", &self.context.is_some())
            .finish()
    }
}
