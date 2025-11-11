// threadcity-ui/src/ui_logger.rs
// Módulo para redirigir println! hacia la terminal del UI

use gtk::TextBuffer;
use gtk::prelude::*; // <-- IMPORTANTE: habilita TextBufferExt (insert, end_iter)
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct UiLogger {
    buffer: Rc<RefCell<TextBuffer>>,
}

impl UiLogger {
    pub fn init(buffer: TextBuffer) -> Self {
        Self {
            buffer: Rc::new(RefCell::new(buffer)),
        }
    }

    pub fn log(&self, msg: &str) {
        let buffer_ref = self.buffer.borrow();
        // end_iter() devuelve un TextIter por valor -> crear variable mutable
        let mut iter = buffer_ref.end_iter();
        buffer_ref.insert(&mut iter, &format!("{}\n", msg));
        println!("{}", msg); // también sigue saliendo en consola
    }
}
