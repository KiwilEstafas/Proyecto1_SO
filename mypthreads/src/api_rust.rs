//! api rust amigable para demos y simulacion
//! wrappers que operan sobre una instancia de runtime

use crate::runtime::ThreadRuntime;
use crate::signals::ThreadSignal;
use crate::thread::{SchedulerType, ThreadEntry, ThreadId, ThreadState};
use crate::mutex::MyMutex;

pub fn my_thread_create(
    rt: &mut ThreadRuntime,
    name: &str,
    sched: SchedulerType,
    entry: ThreadEntry,
    tickets: Option<u32>,
    deadline: Option<u64>,
) -> ThreadId {
    rt.spawn(name, sched, entry, tickets, deadline)
}

pub fn my_thread_end() -> ThreadSignal {
    ThreadSignal::Exit
}

pub fn my_thread_yield() -> ThreadSignal {
    ThreadSignal::Yield
}

// bloquea al hilo actual hasta que el objetivo termine
// si el objetivo ya termino devuelve yield
// si el objetivo esta detached devuelve yield
// si el objetivo no existe devuelve yield
pub fn my_thread_join(rt: &mut ThreadRuntime, target: ThreadId) -> ThreadSignal {
    let Some(self_tid) = rt.current() else {
        return ThreadSignal::Yield;
    };

    let mut should_block = false;
    let mut can_join = true;

    if let Some(t) = rt.threads.get(&target) {
        if t.detached {
            can_join = false;
        } else if t.state != ThreadState::Terminated {
            should_block = true;
        }
    } else {
        can_join = false;
    }

    if !can_join {
        return ThreadSignal::Yield;
    }

    if should_block {
        if let Some(t) = rt.threads.get_mut(&target) {
            if !t.joiners.contains(&self_tid) {
                t.joiners.push(self_tid);
            }
        }
        return ThreadSignal::Block;
    }

    ThreadSignal::Yield
}

// marca un hilo como detached y limpia si ya termino
pub fn my_thread_detach(rt: &mut ThreadRuntime, target: ThreadId) {
    if let Some(t) = rt.threads.get_mut(&target) {
        t.detached = true;
        if t.state == ThreadState::Terminated {
            rt.threads.remove(&target);
        }
    }
}

pub fn my_mutex_lock(rt: &mut ThreadRuntime, mutex: &mut MyMutex) -> ThreadSignal{
    let Some(tid) = rt.current() else {
        return  ThreadSignal::Yield;
    };

    if mutex.my_mutex_lock(tid){
        ThreadSignal::Block
    } else {
        ThreadSignal::Continue
    }
}

pub fn my_mutex_unlock(rt: &mut ThreadRuntime, mutex: &mut MyMutex) -> ThreadSignal{
    let Some(tid) = rt.current() else {
        return ThreadSignal::Yield;
    };

    if let Some(next_tid) = mutex.my_mutex_unlock(tid){
        rt.wake(next_tid);
    }
    ThreadSignal::Continue
    
}

pub fn my_mutex_trylock(rt: &mut ThreadRuntime, mutex: &mut MyMutex) -> ThreadSignal {
    let Some(tid) = rt.current() else {
        return ThreadSignal::Yield;
    };

    if mutex.my_mutex_trylock(tid){
        ThreadSignal::Continue
    } else {
        ThreadSignal::Yield
    }

}

pub fn my_thread_chsched( rt: &mut ThreadRuntime, target: ThreadId, new_sched: SchedulerType, tickets: Option<u32>, deadline: Option<u64>) -> bool {
    if let Some(t) = rt.threads.get_mut(&target) {
        t.sched_type = new_sched;

        if let Some(tix) = tickets{
            t.tickets = tix;
        }

        if let Some(dl) = deadline{
            t.deadline = Some(dl);
        }

        true 
    } else {
        false
    }
}