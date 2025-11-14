use crate::mypthreads_api::{
    my_mutex_destroy, my_mutex_init, my_mutex_lock, my_mutex_trylock, my_mutex_unlock, MyMutex,
};
use crate::signals::ThreadSignal;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Celda protegida por un MyMutex
pub struct MyMutexCell<T> {
    mtx: MyMutex,
    data: UnsafeCell<T>,
}

// SAFETY: MyMutexCell es Send si T es Send (el mutex protege el acceso)
unsafe impl<T: Send> Send for MyMutexCell<T> {}

// SAFETY: MyMutexCell es Sync si T es Send (acceso exclusivo vía mutex)
unsafe impl<T: Send> Sync for MyMutexCell<T> {}

impl<T> MyMutexCell<T> {
    /// Crea un nuevo MyMutexCell con el valor dado
    pub fn new(value: T) -> Self {
        Self {
            mtx: my_mutex_init(),
            data: UnsafeCell::new(value),
        }
    }
    /// Solicitar el lock.   
    pub fn request_lock(&self) -> ThreadSignal {
        my_mutex_lock(&self.mtx)
    }

    /// Libera el lock directamente.
    pub fn force_unlock_for_main(&self) {
        self.mtx.force_unlock();
    }

    /// Entrar a la sección crítica (después de que el runtime otorgó el lock).
    pub fn enter(&self) -> MyGuard<'_, T> {
        MyGuard {
            cell: self,
            _no_send: std::marker::PhantomData,
        }
    }

    /// Liberar el lock.
    pub fn request_unlock(&self) -> ThreadSignal {
        my_mutex_unlock(&self.mtx)
    }

    /// Intenta entrar a la sección crítica sin bloquear.
    pub fn try_enter(&self) -> Option<MyGuard<'_, T>> {
        let ok = my_mutex_trylock(&self.mtx);
        // println!("try_enter: try_lock returned {}", ok);
        if ok {
            Some(MyGuard {
                cell: self,
                _no_send: std::marker::PhantomData,
            })
        } else {
            None
        }
    }
}

impl<T> Drop for MyMutexCell<T> {
    fn drop(&mut self) {
        // Destruir el mutex cuando se destruye el cell
        my_mutex_destroy(&mut self.mtx);
    }
}

/// Guard para acceso exclusivo a los datos dentro de MyMutexCell
pub struct MyGuard<'a, T> {
    cell: &'a MyMutexCell<T>,
    _no_send: std::marker::PhantomData<*const ()>, // Hace que Guard no sea Send
}

impl<T> Deref for MyGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: El guard solo existe cuando tenemos el lock
        unsafe { &*self.cell.data.get() }
    }
}

impl<T> DerefMut for MyGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: El guard solo existe cuando tenemos el lock
        unsafe { &mut *self.cell.data.get() }
    }
}


/// Tipo conveniente para compartir MyMutexCell entre hilos
pub type Shared<T> = Arc<MyMutexCell<T>>;

/// Función helper para crear un Shared
pub fn shared<T>(value: T) -> Shared<T> {
    Arc::new(MyMutexCell::new(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let cell = MyMutexCell::new(42);

        // try_enter debería funcionar sin contexto de hilo
        if let Some(mut guard) = cell.try_enter() {
            assert_eq!(*guard, 42);
            *guard = 100;
            drop(guard); // Drop no hace unlock

            // Manualmente liberar
            cell.request_unlock();
        }
    }

    #[test]
    fn test_shared_creation() {
        let shared_cell = shared(vec![1, 2, 3]);

        if let Some(guard) = shared_cell.try_enter() {
            assert_eq!(guard.len(), 3);
            drop(guard);
            shared_cell.request_unlock();
        }
    }
}
