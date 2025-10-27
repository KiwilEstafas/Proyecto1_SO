//! capa con firmas equivalentes a pthreads
//! expone funciones extern c que operan sobre el mismo runtime interno
//! esto permite compatibilidad con firmas tipo pthreads sin romper la api rust
//! ESTO SE VA A ELIMINAR A FUTURO POSIBLEMENTEEEEEEE

use std::cell::RefCell;
use std::ffi::c_void;

use crate::mutex::MyMutex;
use crate::runtime::ThreadRuntime;
use crate::signals::ThreadSignal;
use crate::thread::{SchedulerType, ThreadEntry, ThreadId, ThreadState, RetVal};

thread_local! {
    static RUNTIME: RefCell<ThreadRuntime> = RefCell::new(ThreadRuntime::new());
}

// helper para acceder mut al runtime
fn with_rt<R>(f: impl FnOnce(&mut ThreadRuntime) -> R) -> R {
    RUNTIME.with(|rc| {
        let mut rt = rc.borrow_mut();
        f(&mut *rt)
    })
}

// tipos opacos equivalentes
#[allow(non_camel_case_types)]
pub type my_thread_t = ThreadId;
#[allow(non_camel_case_types)]
pub type my_thread_attr_t = ();
#[allow(non_camel_case_types)]
pub type my_mutex_t = MyMutex;
#[allow(non_camel_case_types)]
pub type my_mutexattr_t = ();

// crea un hilo con firma equivalente a pthread_create
#[unsafe(no_mangle)]
pub extern "C" fn my_thread_create(
    thread: *mut my_thread_t,
    _attr: *const my_thread_attr_t,
    start_routine: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> i32 {
    if thread.is_null() {
        return 1;
    }

    let arg_ptr = arg;

    // wrapper de una sola ejecucion estilo pthread
    let wrapper: ThreadEntry = Box::new(move |rt, tid| {
        let ret = start_routine(arg_ptr);
        if let Some(t) = rt.threads.get_mut(&tid) {
            t.return_value = Some(RetVal(ret));
        }
        ThreadSignal::Exit
    });

    let tid = with_rt(|rt| rt.spawn("thread", SchedulerType::RoundRobin, wrapper, None, None));
    unsafe { *thread = tid; }
    0
}

// termina el hilo actual guardando el valor opaco de retorno
#[unsafe(no_mangle)]
pub extern "C" fn my_thread_end(retval: *mut c_void) {
    with_rt(|rt| {
        if let Some(tid) = rt.current() {
            if let Some(t) = rt.threads.get_mut(&tid) {
                t.return_value = Some(RetVal(retval));
                t.entry = Some(Box::new(|_rt, _tid| ThreadSignal::Exit));
            }
        }
    });
}

// join bloqueante con firma equivalente a pthread_join
#[unsafe(no_mangle)]
pub extern "C" fn my_thread_join(thread: my_thread_t, retval: *mut *mut c_void) -> i32 {
    let retval_ptr = retval;
    let join_target = thread;

    // hilo joiner cooperativo
    let waiter: ThreadEntry = Box::new(move |rt, self_tid| {
        let Some(t) = rt.threads.get(&join_target) else {
            return ThreadSignal::Exit;
        };
        if t.detached {
            return ThreadSignal::Exit;
        }
        if t.state != ThreadState::Terminated {
            if let Some(t) = rt.threads.get_mut(&join_target) {
                if !t.joiners.contains(&self_tid) {
                    t.joiners.push(self_tid);
                }
            }
            return ThreadSignal::Block;
        }
        if !retval_ptr.is_null() {
            unsafe {
                *retval_ptr = t.return_value.as_ref().map(|v| v.0).unwrap_or(std::ptr::null_mut());
            }
        }
        ThreadSignal::Exit
    });

    // crear el joiner
    with_rt(|rt| {
        let _joiner_tid = rt.spawn("joiner", SchedulerType::RoundRobin, waiter, None, None);
    });

    // bombear la cola ready hasta que no quede trabajo
    loop {
        let had_ready = with_rt(|rt| {
            if rt.ready.is_empty() {
                false
            } else {
                rt.run_once();
                true
            }
        });
        if !had_ready {
            break;
        }
    }

    0
}

// detach equivalente a pthread_detach
#[unsafe(no_mangle)]
pub extern "C" fn my_thread_detach(thread: my_thread_t) -> i32 {
    with_rt(|rt| {
        if let Some(t) = rt.threads.get_mut(&thread) {
            t.detached = true;
            if t.state == ThreadState::Terminated {
                rt.threads.remove(&thread);
            }
            0
        } else {
            1
        }
    })
}

// yield cooperativo basico
#[unsafe(no_mangle)]
pub extern "C" fn my_thread_yield() -> i32 {
    0
}

// mutex init equivalente a pthread_mutex_init
#[unsafe(no_mangle)]
pub extern "C" fn my_mutex_init(mutex: *mut my_mutex_t, _attr: *const my_mutexattr_t) -> i32 {
    if mutex.is_null() {
        return 1;
    }
    unsafe { std::ptr::write(mutex, MyMutex::my_mutex_init()); }
    0
}

// mutex destroy equivalente a pthread_mutex_destroy
#[unsafe(no_mangle)]
pub extern "C" fn my_mutex_destroy(mutex: *mut my_mutex_t) -> i32 {
    if mutex.is_null() {
        return 1;
    }
    unsafe {
        let m = &mut *mutex;
        m.my_mutex_destroy()
    }
}

