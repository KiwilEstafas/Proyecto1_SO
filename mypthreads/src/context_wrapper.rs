// wrapper minimalista sobre el crate context
// proporciona una interfaz segura para crear y cambiar contextos

use context::{Context, Transfer};
use context::stack::ProtectedFixedSizeStack;

const STACK_SIZE: usize = 8192; // 8kb por hilo

// estructura que encapsula el contexto y la pila de un hilo
pub struct ThreadContext {
    context: Option<Context>,
    // usamos box para que la pila viva en el heap y no se mueva
    _stack: Box<ProtectedFixedSizeStack>,
}

impl ThreadContext {
    // crea un nuevo contexto con su propia pila
    // entry: funcion que se ejecutara cuando este hilo arranque
    pub fn new(entry: extern "C" fn(Transfer) -> !) -> Self {
        // allocar una pila protegida de tamano fijo en el heap
        let stack = ProtectedFixedSizeStack::new(STACK_SIZE)
            .expect("no se pudo crear la pila");
        
        // safety: creamos un contexto apuntando a una funcion valida
        // el stack es propiedad de este struct y vivira mientras el contexto exista
        let context = unsafe {
            Context::new(&stack, entry)
        };
        
        Self {
            context: Some(context),
            _stack: Box::new(stack),
        }
    }

    // transfiere control de este contexto (self) a otro contexto (target)
    // guarda el estado actual y restaura el estado del target
    // cuando target haga resume de vuelta, volveremos aqui
    pub unsafe fn resume(&mut self, target: &mut ThreadContext) -> usize {
        // extraer el contexto del target
        let target_ctx = target.context.take()
            .expect("target context debe existir");
        
        // hacer la transferencia
        // esto guarda nuestro estado y salta al target
        let transfer = target_ctx.resume(0);
        
        // cuando volvamos aqui, guardamos el contexto que nos devolvio
        // y extraemos el dato antes de guardar el contexto
        let data = transfer.data;
        self.context = Some(transfer.context);
        
        data
    }

    // crea un contexto especial para el runtime principal
    // este contexto no necesita una funcion de entrada porque ya esta corriendo
    pub fn new_runtime() -> Self {
        // el runtime no necesita un contexto real al inicio
        // solo necesita poder recibir transferencias
        // creamos un stack dummy que nunca se usara realmente
        let stack = ProtectedFixedSizeStack::new(STACK_SIZE)
            .expect("no se pudo crear la pila del runtime");
        
        Self {
            context: None, // se llenara cuando recibamos la primera transferencia
            _stack: Box::new(stack),
        }
    }
}

// necesitamos que threadcontext se pueda debuggear para facilitar desarrollo
impl std::fmt::Debug for ThreadContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadContext")
            .field("has_context", &self.context.is_some())
            .finish()
    }
}
