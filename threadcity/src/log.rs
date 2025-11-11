// threadcity/src/log.rs
// Logger mínimo sin std::sync::Mutex. Por defecto imprime a consola.
// Se puede redirigir con set_logger(fn(&str)) antes de correr la simulación.

use core::sync::atomic::{AtomicPtr, Ordering};

type LogFn = fn(&str);

fn default_log(s: &str) {
    // Comportamiento actual: consola
    println!("{}", s);
}

// Almacena un puntero a función; sin Mutex. Se asume set_logger() se llama antes de uso concurrente.
static LOGGER_PTR: AtomicPtr<()> = AtomicPtr::new(default_log as *mut ());

#[inline]
pub fn set_logger(f: LogFn) {
    LOGGER_PTR.store(f as *mut (), Ordering::Relaxed);
}

#[inline]
pub fn log_str(s: &str) {
    let p = LOGGER_PTR.load(Ordering::Relaxed);
    let f: LogFn = unsafe { core::mem::transmute(p) };
    f(s);
}

#[macro_export]
macro_rules! tc_log {
    ($($arg:tt)*) => {{
        $crate::log::log_str(&format!($($arg)*));
    }};
}
