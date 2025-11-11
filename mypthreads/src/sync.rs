use crate::mypthreads_api::{
    my_mutex_destroy, my_mutex_init, my_mutex_lock, my_mutex_trylock, my_mutex_unlock, MyMutex,
};
use crate::signals::ThreadSignal;

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Contenedor thread-safe para datos compartidos usando MyMutex.
///
/// A diferencia de std::sync::Mutex, este wrapper NO usa RAII para unlock
/// porque my_mutex_unlock retorna ThreadSignal que debe ser devuelto al runtime.
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

    // ═══════════════════════════════════════════════════════════════════
    // CAMINO COOPERATIVO (con señales ThreadSignal)
    // ═══════════════════════════════════════════════════════════════════

    /// Paso 1: Solicitar el lock.
    ///
    /// IMPORTANTE: El ThreadSignal retornado DEBE ser devuelto al runtime.
    ///
    /// - Si retorna `ThreadSignal::MutexLock(addr)`: el hilo NO tiene el lock aún,
    ///   el runtime lo bloqueará y lo reanudará cuando se otorgue el lock.
    /// - Si retorna `ThreadSignal::Continue`: el lock se adquirió inmediatamente,
    ///   puede proceder a llamar `enter()`.
    pub fn request_lock(&self) -> ThreadSignal {
        my_mutex_lock(&self.mtx)
    }

    /// Libera el lock directamente.
    ///
    /// IMPORTANTE: Esta función es SÓLO para ser usada por el hilo `main`.
    /// Bypassea el sistema de señales porque el `main` no es un hilo gestionado.
    /// Asume que el `main` thread tiene el tid `0`.
    pub fn force_unlock_for_main(&self) {
        self.mtx.internal.force_unlock();
    }

    /// Paso 2: Entrar a la sección crítica (después de que el runtime otorgó el lock).
    ///
    /// PRECONDICIÓN: El hilo debe haber sido reanudado por el runtime después de
    /// que `request_lock()` retornó `MutexLock`, O haber recibido `Continue`.
    ///
    /// Retorna un guard que permite acceso mutable a los datos.
    ///
    /// IMPORTANTE: Este guard NO libera el lock en Drop. Debe llamarse
    /// explícitamente a `request_unlock()` cuando termine.
    pub fn enter(&self) -> MyGuard<'_, T> {
        MyGuard {
            cell: self,
            _no_send: std::marker::PhantomData,
        }
    }

    /// Paso 3: Liberar el lock.
    ///
    /// IMPORTANTE: El ThreadSignal retornado DEBE ser devuelto al runtime.
    ///
    /// Retorna generalmente `ThreadSignal::Continue`, pero el caller debe
    /// devolverlo al runtime para mantener el contrato correcto.
    pub fn request_unlock(&self) -> ThreadSignal {
        my_mutex_unlock(&self.mtx)
    }

    // ═══════════════════════════════════════════════════════════════════
    // CAMINO NO BLOQUEANTE (sin señales ThreadSignal)
    // ═══════════════════════════════════════════════════════════════════

    /// Intenta adquirir el lock sin bloquearse.
    ///
    /// - Si retorna `Some(guard)`: el lock fue adquirido, puede usar el guard
    /// - Si retorna `None`: el lock no está disponible
    ///
    /// IMPORTANTE: Cuando termine de usar el guard, debe llamarse explícitamente
    /// a `request_unlock()` para liberar el lock.
    ///
    /// Nota: Este método usa `my_mutex_trylock` internamente, que NO requiere
    /// contexto de hilo inicializado (puede llamarse desde cualquier hilo).
    pub fn try_enter(&self) -> Option<MyGuard<'_, T>> {
        let ok = my_mutex_trylock(&self.mtx);
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

/// Guard que proporciona acceso a los datos protegidos.
///
/// CRÍTICO: Este guard NO libera el lock en Drop.
/// El caller debe llamar explícitamente a `request_unlock()` para liberar.
///
/// Esto es necesario porque `my_mutex_unlock()` retorna un `ThreadSignal`
/// que debe ser devuelto al runtime, y no podemos hacer eso desde `Drop`.
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

// NO implementamos Drop para MyGuard
// El unlock debe ser explícito para no perder el ThreadSignal

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