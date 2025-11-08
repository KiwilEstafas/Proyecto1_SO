use crate::thread::{ThreadId, SchedulerType, MyThread};
use std::collections::{HashMap, VecDeque};
use rand::Rng;



/// SCHEDULER DE TIEMPO REAL: Encuentra el hilo listo con el deadline más cercano.
fn schedule_real_time<'a>(
    ready_queue: &'a VecDeque<ThreadId>,
    threads: &'a HashMap<ThreadId, MyThread>
) -> Option<ThreadId> {
    ready_queue.iter()
        .filter_map(|&tid| {
            let thread = threads.get(&tid)?;
            if thread.sched_type == SchedulerType::RealTime {
                Some((tid, thread.deadline.unwrap_or(u64::MAX)))
            } else {
                None
            }
        })
        .min_by_key(|&(_, deadline)| deadline)
        .map(|(tid, _)| tid)
}

/// SCHEDULER DE SORTEO: Elige un ganador basado en tiquetes entre los hilos que no son de tiempo real.
fn schedule_lottery<'a>(
    ready_queue: &'a VecDeque<ThreadId>,
    threads: &'a HashMap<ThreadId, MyThread>
) -> Option<ThreadId> {
    
    // Filtramos solo los candidatos para sorteo (Lottery y RoundRobin).
    let lottery_candidates: Vec<ThreadId> = ready_queue.iter()
        .filter(|&&tid| threads.get(&tid).map_or(false, |t| t.sched_type != SchedulerType::RealTime))
        .cloned()
        .collect();

    if lottery_candidates.is_empty() {
        return None;
    }
    
    let total_tickets = lottery_candidates.iter()
        .map(|&tid| threads.get(&tid).unwrap().tickets)
        .sum();

    if total_tickets == 0 {
        // Si no hay tiquetes, no podemos hacer sorteo.
        return None;
    }

    let winning_ticket = rand::rng().random_range(1..=total_tickets);
    let mut accumulated_tickets = 0;

    for &tid in &lottery_candidates {
        accumulated_tickets += threads.get(&tid).unwrap().tickets;
        if accumulated_tickets >= winning_ticket {
            return Some(tid);
        }
    }
    
    None // No debería pasar si hay tiquetes.
}

/// SCHEDULER ROUND ROBIN: Simplemente toma el primer hilo que no sea de tiempo real.
fn schedule_round_robin<'a>(
    ready_queue: &'a VecDeque<ThreadId>,
    threads: &'a HashMap<ThreadId, MyThread>
) -> Option<ThreadId> {
    ready_queue.iter()
        .find(|&&tid| threads.get(&tid).map_or(false, |t| t.sched_type != SchedulerType::RealTime))
        .copied()
}


/// La función principal que maneja los schedulers según su prioridad.
pub fn select_next_thread(
    ready_queue: &VecDeque<ThreadId>,
    threads: &HashMap<ThreadId, MyThread>,
    now_ms: u64,
) -> Option<ThreadId> {
    
    if ready_queue.is_empty() {
        return None;
    }

    // 1. MÁXIMA PRIORIDAD: RT
    if let Some(tid) = schedule_real_time(ready_queue, threads) {
        let deadline = threads.get(&tid).unwrap().deadline.unwrap_or(0);
        if deadline < now_ms {
            println!("[Scheduler] ¡¡¡FALLO DE TIEMPO REAL!!! Hilo {} falló su deadline {}. Tiempo actual: {}", tid, deadline, now_ms);
        }
        println!("[Scheduler] TIEMPO REAL: Seleccionado hilo {}", tid);
        return Some(tid);
    }

    // 2. PRIORIDAD NORMAL: Si no hay hilos de tiempo real, realizamos un sorteo.
    if let Some(tid) = schedule_lottery(ready_queue, threads) {
        println!("[Scheduler] SORTEO: Seleccionado hilo {}", tid);
        return Some(tid);
    }

    // 3. FALLBACK: Si el sorteo falla (ej. 0 tiquetes), usamos Round Robin simple.
    if let Some(tid) = schedule_round_robin(ready_queue, threads) {
        println!("[Scheduler] ROUND ROBIN (Fallback): Seleccionado hilo {}", tid);
        return Some(tid);
    }
    
    // 4. FALLBACK EXTREMO: Si solo hay hilos de tiempo real pero la función RT falló,
    // simplemente tomamos el primero que haya en la cola.
    ready_queue.front().copied()
}