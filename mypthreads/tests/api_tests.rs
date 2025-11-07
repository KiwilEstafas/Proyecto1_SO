use mypthreads::mypthreads_api::*;
use mypthreads::mypthreads_api::SchedulerParams;
use mypthreads::signals::ThreadSignal;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// La prueba de JOIN ya funciona, la dejamos como está.
#[test]
fn test_api_join_waits_for_thread_to_finish() {
    println!("\n=== TEST API: my_thread_join ===");
    let execution_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let log_clone_worker = execution_log.clone();
    let log_clone_waiter = execution_log.clone();
    let worker_tid = my_thread_create("Worker",SchedulerParams::RoundRobin,Box::new(move |_| {
            let mut log = log_clone_worker.lock().unwrap();
            if log.iter().filter(|s| s.starts_with("Worker")).count() == 0 {
                log.push("Worker: Iniciando trabajo".to_string());
                println!("  [Worker] Trabajando...");
                return ThreadSignal::Yield;
            } else {
                log.push("Worker: Trabajo terminado".to_string());
                println!("  [Worker] Terminado.");
                return ThreadSignal::Exit;
            }
        }),
    );
    my_thread_create("Waiter",SchedulerParams::RoundRobin,Box::new(move |_| {
            let mut log = log_clone_waiter.lock().unwrap();
            if !log.contains(&"Waiter: Esperando a Worker".to_string()) {
                log.push("Waiter: Esperando a Worker".to_string());
                println!("  [Waiter] Voy a esperar por el hilo {}.", worker_tid);
                return my_thread_join(worker_tid);
            } else {
                log.push("Waiter: Continuo ejecución".to_string());
                println!("  [Waiter] El Worker ha terminado. Ahora yo termino.");
                return ThreadSignal::Exit;
            }
        }),
    );
    run_simulation(10);
    let log = execution_log.lock().unwrap();
    println!("\nLog de ejecución final: {:?}", log);
    let worker_finish_pos = log.iter().position(|s| s == "Worker: Trabajo terminado").unwrap();
    let waiter_continue_pos = log.iter().position(|s| s == "Waiter: Continuo ejecución").unwrap();
    assert!(worker_finish_pos < waiter_continue_pos, "¡Join falló! El Waiter no esperó al Worker.");
    println!("\n  ✓ Test pasado: my_thread_join funciona correctamente.");
}

/// Test para Mutex: CORREGIDO con una máquina de estados interna y robusta.
#[test]
fn test_api_mutex_provides_mutual_exclusion() {
    println!("\n=== TEST API: my_mutex_lock / my_mutex_unlock ===");
    
    let shared_mutex = Arc::new(my_mutex_init());
    let shared_counter = Arc::new(Mutex::new(0));
    let execution_log = Arc::new(Mutex::new(VecDeque::<String>::new()));

    for i in 0..2 {
        let mutex_clone = shared_mutex.clone();
        let counter_clone = shared_counter.clone();
        let log_clone = execution_log.clone();
        // Cada hilo tiene su propio contador de estado.
        let state = Arc::new(Mutex::new(0));

        my_thread_create(
            &format!("MutexUser-{}", i),
            SchedulerParams::RoundRobin,
            Box::new(move |_| {
                let mut state_val = state.lock().unwrap();
                
                match *state_val {
                    0 => { // Estado inicial: pedir el lock.
                        *state_val = 1;
                        println!("  [Hilo {}] Intentando tomar el mutex...", i);
                        return my_mutex_lock(&mutex_clone);
                    }
                    1 => { // Despertado: ya tenemos el lock. Trabajar y ceder.
                        *state_val = 2;
                        println!("  [Hilo {}] Mutex adquirido. Entrando a sección crítica.", i);
                        log_clone.lock().unwrap().push_back(format!("Hilo {} ENTRA", i));
                        *counter_clone.lock().unwrap() += 1;
                        return ThreadSignal::Yield;
                    }
                    2 => { // Despertado de nuevo: liberar el lock.
                        *state_val = 3;
                        println!("  [Hilo {}] Saliendo de sección crítica y liberando mutex.", i);
                        log_clone.lock().unwrap().push_back(format!("Hilo {} SALE", i));
                        return my_mutex_unlock(&mutex_clone);
                    }
                    _ => { // Ya liberamos, ahora terminar.
                        return ThreadSignal::Exit;
                    }
                }
            }),
        );
    }
    
    run_simulation(20);

    assert_eq!(*shared_counter.lock().unwrap(), 2, "El contador debe ser 2.");

    let log = execution_log.lock().unwrap();
    println!("\nOrden de acceso a la sección crítica: {:?}", log);
    
    assert_eq!(&log[0], "Hilo 0 ENTRA");
    assert_eq!(&log[1], "Hilo 0 SALE");
    assert_eq!(&log[2], "Hilo 1 ENTRA");
    assert_eq!(&log[3], "Hilo 1 SALE");

    println!("\n  ✓ Test pasado: Mutex garantiza la exclusión mutua.");
}

/// Test para my_mutex_trylock: CORREGIDO con una máquina de estados interna y robusta.
#[test]
fn test_api_mutex_trylock() {
    println!("\n=== TEST API: my_mutex_trylock ===");

    let shared_mutex = Arc::new(my_mutex_init());
    let log = Arc::new(Mutex::new(Vec::<String>::new()));

    // Hilo Locker
    my_thread_create(
        "Locker",
        SchedulerParams::RoundRobin,
        Box::new({
            let log_a = log.clone();
            let mutex_a = shared_mutex.clone();
            let state = Arc::new(Mutex::new(0));
            move |_| {
                let mut state_val = state.lock().unwrap();
                match *state_val {
                    0 => { // Pedir el lock
                        *state_val = 1;
                        println!("  [Locker] Intentando tomar el lock...");
                        return my_mutex_lock(&mutex_a);
                    }
                    1 => { // Ya tenemos el lock, ceder para que el otro pruebe
                        *state_val = 2;
                        log_a.lock().unwrap().push("Locker: Lock adquirido".to_string());
                        println!("  [Locker] Lock adquirido, cediendo...");
                        return ThreadSignal::Yield;
                    }
                    2 => { // Liberar el lock
                        *state_val = 3;
                        log_a.lock().unwrap().push("Locker: Lock liberado".to_string());
                        println!("  [Locker] Liberando lock...");
                        return my_mutex_unlock(&mutex_a);
                    }
                    _ => return ThreadSignal::Exit,
                }
            }
        }),
    );

    // Hilo TryLocker
    my_thread_create(
        "TryLocker",
        SchedulerParams::RoundRobin,
        Box::new({
            let log_b = log.clone();
            let mutex_b = shared_mutex.clone();
            let state = Arc::new(Mutex::new(0));
            move |_| {
                let mut state_val = state.lock().unwrap();
                match *state_val {
                    0 => { // Ceder para que Locker actúe primero
                        *state_val = 1;
                        return ThreadSignal::Yield;
                    }
                    1 => { // Probar trylock (debería fallar) y ceder de nuevo
                        *state_val = 2;
                        println!("  [TryLocker] Intentando tomar el lock con trylock (debería fallar)...");
                        if !my_mutex_trylock(&mutex_b) {
                            println!("  [TryLocker] ¡Correcto! trylock falló.");
                            log_b.lock().unwrap().push("TryLocker: trylock falló".to_string());
                        } else {
                            my_mutex_unlock(&mutex_b);
                            panic!("trylock debería haber fallado!");
                        }
                        return ThreadSignal::Yield;
                    }
                    2 => { // Probar trylock de nuevo (debería funcionar)
                        *state_val = 3;
                        println!("  [TryLocker] Intentando de nuevo con trylock (debería funcionar)...");
                        if my_mutex_trylock(&mutex_b) {
                            println!("  [TryLocker] ¡Correcto! trylock tuvo éxito.");
                            log_b.lock().unwrap().push("TryLocker: trylock tuvo éxito".to_string());
                            my_mutex_unlock(&mutex_b);
                        } else {
                            panic!("trylock debería haber tenido éxito!");
                        }
                        return ThreadSignal::Exit;
                    }
                    _ => return ThreadSignal::Exit,
                }
            }
        }),
    );
    
    run_simulation(20);
    
    let final_log = log.lock().unwrap();
    println!("\nLog de eventos de trylock: {:?}", final_log);

    assert_eq!(&final_log[0], "Locker: Lock adquirido");
    assert_eq!(&final_log[1], "TryLocker: trylock falló");
    assert_eq!(&final_log[2], "Locker: Lock liberado");
    assert_eq!(&final_log[3], "TryLocker: trylock tuvo éxito");
    
    println!("\n  ✓ Test pasado: my_mutex_trylock funciona como se esperaba.");
}