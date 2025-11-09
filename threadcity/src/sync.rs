// threadcity/src/sync.rs
// Wrappers ergonómicos sobre MyMutex para uso en la simulación

use mypthreads::mypthreads_api::{my_mutex_init, my_mutex_lock, my_mutex_unlock, MyMutex};
use mypthreads::ThreadSignal;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Wrapper thread-safe sobre MyMutex que proporciona una API similar a std::sync::Mutex
/// 
/// IMPORTANTE: Este wrapper está diseñado para uso DENTRO de hilos mypthreads.
/// La operación `lock()` puede causar que el hilo se bloquee usando ThreadSignal::Block
pub struct SharedMutex<T> {
    inner: MyMutex,
    data: UnsafeCell<T>,
}

// SAFETY: SharedMutex es Send si T es Send porque el mutex protege el acceso
unsafe impl<T: Send> Send for SharedMutex<T> {}
// SAFETY: SharedMutex es Sync porque proporciona acceso exclusivo mediante el mutex
unsafe impl<T: Send> Sync for SharedMutex<T> {}

impl<T> SharedMutex<T> {
    /// Crea un nuevo SharedMutex con el valor dado
    pub fn new(value: T) -> Self {
        Self {
            inner: my_mutex_init(),
            data: UnsafeCell::new(value),
        }
    }

    /// Adquiere el mutex, bloqueando el hilo si es necesario
    /// 
    /// NOTA: Esta función retorna ThreadSignal para que el hilo pueda
    /// comunicarse con el runtime si necesita bloquearse
    pub fn lock(&self) -> (SharedMutexGuard<T>, ThreadSignal) {
        let signal = my_mutex_lock(&self.inner);
        
        // El guard se crea siempre, pero el ThreadSignal indica si debemos bloquearnos
        let guard = SharedMutexGuard {
            mutex: self,
            _no_send: std::marker::PhantomData,
        };
        
        (guard, signal)
    }

    /// Intenta adquirir el mutex sin bloquearse
    /// Retorna Some(guard) si tuvo éxito, None si el mutex está ocupado
    pub fn try_lock(&self) -> Option<SharedMutexGuard<T>> {
        if mypthreads::mypthreads_api::my_mutex_trylock(&self.inner) {
            Some(SharedMutexGuard {
                mutex: self,
                _no_send: std::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Acceso directo a los datos (UNSAFE - solo usar si ya tienes el lock)
    /// 
    /// # Safety
    /// El caller debe garantizar que tiene acceso exclusivo al mutex
    #[allow(dead_code)]
    unsafe fn get_unchecked(&self) -> &mut T {
        &mut *self.data.get()
    }
}

/// Guard que proporciona acceso a los datos protegidos por el mutex
/// 
/// Cuando se drop, automáticamente libera el mutex
pub struct SharedMutexGuard<'a, T> {
    mutex: &'a SharedMutex<T>,
    _no_send: std::marker::PhantomData<*const ()>, // Hace que Guard no sea Send
}

impl<T> Deref for SharedMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for SharedMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for SharedMutexGuard<'_, T> {
    fn drop(&mut self) {
        // Liberar el mutex cuando se destruye el guard
        my_mutex_unlock(&self.mutex.inner);
    }
}

/// Tipo conveniente para compartir datos protegidos por mutex entre hilos
pub type Shared<T> = Arc<SharedMutex<T>>;

/// Función helper para crear datos compartidos
pub fn shared<T>(value: T) -> Shared<T> {
    Arc::new(SharedMutex::new(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_mutex_creation() {
        let mutex = SharedMutex::new(42);
        
        // Verificar que try_lock funciona
        let guard = mutex.try_lock().expect("Should acquire lock");
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_shared_creation() {
        let shared = shared(vec![1, 2, 3]);
        
        if let Some(guard) = shared.try_lock() {
            assert_eq!(guard.len(), 3);
        }
    }
}