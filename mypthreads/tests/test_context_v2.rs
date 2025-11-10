//! test de cambio de contexto real con ThreadRuntimeV2
//! dos hilos hacen ping-pong usando yield

use mypthreads::runtime::ThreadRuntimeV2;
use mypthreads::signals::ThreadSignal;
use mypthreads::thread::{ContextThreadEntry, SchedulerType};
use std::sync::{Arc, Mutex};

#[test]
fn test_basic_context_switch() {
    println!("\n=== TEST: Cambio de Contexto Básico ===\n");

    let mut rt = ThreadRuntimeV2::new();

    // contador compartido para verificar ejecución
    let counter = Arc::new(Mutex::new(0));

    // Hilo A: Ping
    let counter_a = counter.clone();
    // CORRECCIÓN: La clausura ahora acepta dos argumentos (tid, tickets).
    let thread_a: ContextThreadEntry = Box::new(move |tid, _| {
        let mut count = counter_a.lock().unwrap();
        *count += 1;
        let current = *count;
        drop(count);

        println!("  [Hilo {}] Ping! (ejecución #{})", tid, current);

        if current < 3 {
            ThreadSignal::Yield
        } else {
            println!("  [Hilo {}] Ping terminado", tid);
            ThreadSignal::Exit
        }
    });

    // Hilo B: Pong
    let counter_b = counter.clone();
    // CORRECCIÓN: La clausura ahora acepta dos argumentos.
    let thread_b: ContextThreadEntry = Box::new(move |tid, _| {
        let mut count = counter_b.lock().unwrap();
        *count += 1;
        let current = *count;
        drop(count);

        println!("  [Hilo {}] Pong! (ejecución #{})", tid, current);

        if current < 3 {
            ThreadSignal::Yield
        } else {
            println!("  [Hilo {}] Pong terminado", tid);
            ThreadSignal::Exit
        }
    });

    // Crear hilos
    let tid_a = rt.spawn("Ping", SchedulerType::RoundRobin, thread_a, 1, None);
    let tid_b = rt.spawn("Pong", SchedulerType::RoundRobin, thread_b, 1, None);

    println!("Hilos creados: A={}, B={}\n", tid_a, tid_b);

    // Ejecutar
    rt.run(10);

    // Verificar que ambos ejecutaron
    let final_count = *counter.lock().unwrap();
    println!("\n=== Resultado Final ===");
    println!("Total de ejecuciones: {}", final_count);
    println!("Hilos en ready: {}", rt.ready.len());
    println!("Hilos bloqueados: {}", rt.blocked.len());

    assert!(final_count >= 4, "Los hilos no ejecutaron suficientes veces");
    assert!(rt.ready.is_empty(), "Todavía hay hilos en ready");

    println!("\n  Test pasado: El cambio de contexto funciona!");
}

#[test]
fn test_single_thread_yields() {
    println!("\n=== TEST: Un Solo Hilo con Múltiples Yields ===\n");

    let mut rt = ThreadRuntimeV2::new();

    let executions = Arc::new(Mutex::new(Vec::new()));
    let exec_clone = executions.clone();

    // CORRECCIÓN: La clausura ahora acepta dos argumentos.
    let thread: ContextThreadEntry = Box::new(move |tid, _| {
        exec_clone.lock().unwrap().push(tid);
        let count = exec_clone.lock().unwrap().len();

        println!("  [Hilo {}] Ejecución #{}", tid, count);

        if count < 5 {
            ThreadSignal::Yield
        } else {
            println!("  [Hilo {}] Completado después de {} ejecuciones", tid, count);
            ThreadSignal::Exit
        }
    });

    rt.spawn("Worker", SchedulerType::RoundRobin, thread, 1, None);

    rt.run(10);

    let exec_count = executions.lock().unwrap().len();
    println!("\nTotal de ejecuciones: {}", exec_count);

    assert_eq!(exec_count, 5, "El hilo debería ejecutar exactamente 5 veces");

    println!("  Test pasado: Los yields funcionan correctamente!");
}

#[test]
fn test_immediate_exit() {
    println!("\n=== TEST: Hilo que Termina Inmediatamente ===\n");

    let mut rt = ThreadRuntimeV2::new();

    let executed = Arc::new(Mutex::new(false));
    let exec_clone = executed.clone();

    // CORRECCIÓN: La clausura ahora acepta dos argumentos.
    let thread: ContextThreadEntry = Box::new(move |tid, _| {
        println!("  [Hilo {}] Ejecutando y terminando inmediatamente", tid);
        *exec_clone.lock().unwrap() = true;
        ThreadSignal::Exit
    });

    rt.spawn("QuickExit", SchedulerType::RoundRobin, thread, 1, None);

    rt.run(5);

    assert!(*executed.lock().unwrap(), "El hilo debería haber ejecutado");
    assert!(rt.ready.is_empty(), "No debería haber hilos en ready");

    println!("  Test pasado: Exit inmediato funciona!");
}

#[test]
fn test_three_threads_round_robin() {
    println!("\n=== TEST: Tres Hilos con Round Robin ===\n");

    let mut rt = ThreadRuntimeV2::new();

    let execution_order = Arc::new(Mutex::new(Vec::new()));

    // Crear 3 hilos que cada uno ejecuta 2 veces
    for i in 0..3 {
        let order = execution_order.clone();
        let thread_name = format!("Thread-{}", i);

        // CORRECCIÓN: La clausura ahora acepta dos argumentos.
        let thread: ContextThreadEntry = Box::new(move |tid, _| {
            let mut order_lock = order.lock().unwrap();
            let current_len = order_lock.len();
            order_lock.push((tid, current_len));
            let count = order_lock.iter().filter(|(t, _)| *t == tid).count();
            drop(order_lock);

            println!("  [Hilo {}] Ejecución #{}", tid, count);

            if count < 2 {
                ThreadSignal::Yield
            } else {
                println!("  [Hilo {}] Completado", tid);
                ThreadSignal::Exit
            }
        });

        rt.spawn(&thread_name, SchedulerType::RoundRobin, thread, 1, None);
    }

    rt.run(20);

    let order = execution_order.lock().unwrap();
    println!("\nOrden de ejecución:");
    for (tid, step) in order.iter() {
        println!("  Paso {}: Hilo {}", step, tid);
    }

    assert_eq!(order.len(), 6, "Deberían haber 6 ejecuciones totales (3 hilos × 2)");

    // Verificar que round robin alternó entre hilos
    let first_three_tids: Vec<u32> = order.iter().take(3).map(|(tid, _)| *tid).collect();
    println!("\nPrimeros 3 hilos ejecutados: {:?}", first_three_tids);

    // En Round Robin, los primeros 3 deberían ser diferentes
    let unique_first_three: std::collections::HashSet<_> = first_three_tids.iter().collect();
    assert_eq!(unique_first_three.len(), 3, "Round Robin debería alternar entre hilos");

    println!("  Test pasado: Round Robin funciona correctamente!");
}

#[test]
fn test_context_state_preservation() {
    println!("\n=== TEST: Preservación de Estado entre Yields ===\n");

    let mut rt = ThreadRuntimeV2::new();

    // Usamos un Arc<Mutex<>> para el estado interno del hilo
    let internal_counter = Arc::new(Mutex::new(0));
    let counter_clone = internal_counter.clone();

    // CORRECCIÓN: La clausura ahora acepta dos argumentos.
    let thread: ContextThreadEntry = Box::new(move |tid, _| {
        let mut counter = counter_clone.lock().unwrap();
        *counter += 1;
        let current = *counter;
        drop(counter);

        println!("  [Hilo {}] Contador interno: {}", tid, current);

        if current < 4 {
            ThreadSignal::Yield
        } else {
            println!("  [Hilo {}] Estado preservado correctamente!", tid);
            ThreadSignal::Exit
        }
    });

    rt.spawn("StatefulThread", SchedulerType::RoundRobin, thread, 1, None);

    rt.run(10);

    let final_count = *internal_counter.lock().unwrap();
    assert_eq!(final_count, 4, "El contador debería llegar a 4");

    println!("  Test pasado: El estado se preserva entre cambios de contexto!");
}