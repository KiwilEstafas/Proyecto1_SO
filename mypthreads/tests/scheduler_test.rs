// en un nuevo archivo: tests/unit_scheduler_tests.rs

use mypthreads::sched::select_next_thread;
use mypthreads::thread::{MyThread, SchedulerType};
use std::collections::{HashMap, VecDeque};

// Nota: Para que esto funcione, la struct MyThread y sus campos
// deben ser públicos para que puedas crear instancias en las pruebas.

#[test]
fn unit_test_real_time_is_always_chosen() {
    // 1. Preparación (crear datos de prueba)
    let mut threads = HashMap::new();
    let mut ready_queue = VecDeque::new();

    // Hilo RR
    let rr_thread = MyThread::new(1, "RR".to_string(), SchedulerType::RoundRobin, 1, None, Box::new(|_| unreachable!()));
    threads.insert(1, rr_thread);
    ready_queue.push_back(1);

    // Hilo RT
    let rt_thread = MyThread::new(2, "RT".to_string(), SchedulerType::RealTime, 0, Some(50), Box::new(|_| unreachable!()));
    threads.insert(2, rt_thread);
    ready_queue.push_back(2);

    // 2. Ejecución (llamar a la función que quieres probar)
    let selected_id = select_next_thread(&ready_queue, &threads, 0);

    // 3. Verificación (comprobar que el resultado es el esperado)
    assert_eq!(selected_id, Some(2), "El scheduler de Tiempo Real debería haber sido elegido.");
}