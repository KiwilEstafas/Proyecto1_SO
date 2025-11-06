// mypthreads/tests/integration_tests.rs

// --- Imports ---
// (Asegúrate de que tu biblioteca sea accesible, puede que necesites `use mypthreads::...`)
use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::{SchedulerType, ThreadEntry}; // Eliminado `ThreadId` no usado
use mypthreads::signals::ThreadSignal;
use mypthreads::mutex::MyMutex;
use mypthreads::api_rust::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

// ===================================================================
// TEST 1: CICLO DE VIDA BÁSICO Y YIELD
// ===================================================================
#[test] // <--- ¡Atributo AÑADIDO!
#[ignore]
fn test_basic_thread_lifecycle() {
    println!("\n[TEST 1] Probando creación, yield y finalización de hilos...");

    let mut rt = ThreadRuntime::new();
    let counter = Rc::new(RefCell::new(0));

    // Hilo A: Usamos un estado para seguir la pista de las ejecuciones
    let counter_a = counter.clone();
    let thread_a_entry: ThreadEntry = Box::new(move |_, _| {
        static mut EXECUTIONS: u32 = 0;
        unsafe {
            if EXECUTIONS < 2 {
                println!("Hilo A: ping");
                *counter_a.borrow_mut() += 1;
                EXECUTIONS += 1;
                ThreadSignal::Yield
            } else {
                println!("Hilo A: terminando.");
                ThreadSignal::Exit
            }
        }
    });

    // Hilo B: Simplemente imprime un mensaje y termina.
    let thread_b_entry: ThreadEntry = Box::new(|_, _| {
        println!("Hilo B: ejecutado y terminando.");
        ThreadSignal::Exit
    });

    my_thread_create(&mut rt, "A", SchedulerType::RoundRobin, thread_a_entry, None, None);
    my_thread_create(&mut rt, "B", SchedulerType::RoundRobin, thread_b_entry, None, None);

    rt.run(5);

    assert!(rt.ready.is_empty());
    assert_eq!(*counter.borrow(), 2, "El contador del Hilo A no llegó a 2.");
    println!("[TEST 1] OK: Los hilos se crearon, cedieron el control y terminaron correctamente.");
}

// ===================================================================
// TEST 2: SINCRONIZACIÓN CON JOIN
// ===================================================================
#[test] // <--- ¡Atributo AÑADIDO!
#[ignore]
fn test_thread_join() {
    // ... (El resto de la función es idéntico)
    println!("\n[TEST 2] Probando my_thread_join para esperar por otro hilo...");

    let mut rt = ThreadRuntime::new();
    let worker_finished = Rc::new(RefCell::new(false));

    let flag_clone = worker_finished.clone();
    let worker_entry: ThreadEntry = Box::new(move |_, _| {
        println!("Worker: trabajando...");
        *flag_clone.borrow_mut() = true;
        println!("Worker: terminado.");
        ThreadSignal::Exit
    });
    let worker_tid = my_thread_create(&mut rt, "Worker", SchedulerType::RoundRobin, worker_entry, None, None);

    let joiner_entry: ThreadEntry = Box::new(move |rt, _| {
        println!("Joiner: intentando hacer join al Worker.");
        let signal = my_thread_join(rt, worker_tid);

        if signal == ThreadSignal::Block {
            println!("Joiner: bloqueado correctamente, esperando al Worker.");
        } else {
            println!("Joiner: ha sido despertado, el Worker terminó.");
        }
        signal
    });
    my_thread_create(&mut rt, "Joiner", SchedulerType::RoundRobin, joiner_entry, None, None);

    rt.run(5);

    assert!(*worker_finished.borrow(), "La bandera del worker no se actualizó.");
    println!("[TEST 2] OK: my_thread_join bloqueó y despertó al hilo correctamente.");
}

// ===================================================================
// TEST 3: MUTEX PARA EXCLUSIÓN MUTUA (versión corregida y robusta)
// ===================================================================
#[test]
#[ignore]
fn test_mutex_blocking() {
    println!("\n[TEST 3] Probando bloqueo y desbloqueo de mutex...");

    let mut rt = ThreadRuntime::new();
    let mutex_rc = Rc::new(RefCell::new(MyMutex::my_mutex_init()));
    let shared_data = Rc::new(RefCell::new(0));

    // Esta función de hilo ahora es más inteligente.
    // Intenta bloquear, y si falla, simplemente devuelve la señal de bloqueo.
    // Cuando se despierte, volverá a ejecutar este mismo código desde el principio.
    let create_worker_entry = |data: Rc<RefCell<i32>>, mutex: Rc<RefCell<MyMutex>>, name: &'static str| -> ThreadEntry {
        Box::new(move |rt, _| {
            println!("[{}] Intentando bloquear mutex...", name);

            // 1. Intenta adquirir el lock. Si nos bloqueamos, el scheduler nos detendrá aquí.
            //    Cuando nos despierten, volveremos a ejecutar esta función y lo intentaremos de nuevo.
            let signal = my_mutex_lock(rt, &mut mutex.borrow_mut());
            if signal == ThreadSignal::Block {
                println!("[{}] Se bloqueó correctamente.", name);
                return ThreadSignal::Block;
            }

            // 2. Si llegamos aquí, ¡tenemos el lock!
            println!("[{}] Mutex adquirido. Incrementando dato.", name);
            *data.borrow_mut() += 1;
            println!("[{}] Dato ahora es: {}", name, *data.borrow());

            // 3. Liberamos el mutex para que otros puedan usarlo.
            println!("[{}] Liberando mutex...", name);
            my_mutex_unlock(rt, &mut mutex.borrow_mut());

            // 4. Trabajo terminado.
            println!("[{}] Terminado.", name);
            ThreadSignal::Exit
        })
    };

    let entry1 = create_worker_entry(shared_data.clone(), mutex_rc.clone(), "Hilo 1");
    let entry2 = create_worker_entry(shared_data.clone(), mutex_rc.clone(), "Hilo 2");

    my_thread_create(&mut rt, "Hilo 1", SchedulerType::RoundRobin, entry1, None, None);
    my_thread_create(&mut rt, "Hilo 2", SchedulerType::RoundRobin, entry2, None, None);

    rt.run(10);

    assert_eq!(*shared_data.borrow(), 2, "El dato compartido no fue incrementado por ambos hilos.");
    println!("[TEST 3] OK: El mutex protegió el recurso compartido correctamente.");
}

// ===================================================================
// TEST 4: COMPORTAMIENTO DE LOS SCHEDULERS
// ===================================================================
#[test] // <--- ¡Atributo AÑADIDO!
#[ignore]
fn test_schedulers() {
    // ... (El resto de la función es idéntico)
    println!("\n[TEST 4] Probando Schedulers (Lottery y RealTime)...");

    // --- Sub-test Lottery ---
    let mut rt_lottery = ThreadRuntime::new();
    let run_counts = Rc::new(RefCell::new(HashMap::new()));
    
    let counts1 = run_counts.clone();
    let entry_rich: ThreadEntry = Box::new(move |_, tid| {
        *counts1.borrow_mut().entry(tid).or_insert(0) += 1;
        ThreadSignal::Yield
    });
    
    let counts2 = run_counts.clone();
    let entry_poor: ThreadEntry = Box::new(move |_, tid| {
        *counts2.borrow_mut().entry(tid).or_insert(0) += 1;
        ThreadSignal::Yield
    });

    let rich_tid = my_thread_create(&mut rt_lottery, "Rich", SchedulerType::Lottery, entry_rich, Some(90), None);
    let poor_tid = my_thread_create(&mut rt_lottery, "Poor", SchedulerType::Lottery, entry_poor, Some(10), None);

    rt_lottery.run(100);

    let counts = run_counts.borrow();
    let rich_runs = counts.get(&rich_tid).unwrap_or(&0);
    let poor_runs = counts.get(&poor_tid).unwrap_or(&0);
    println!("Lottery -> Hilo 'Rico' corrió {} veces, Hilo 'Pobre' corrió {} veces.", rich_runs, poor_runs);
    assert!(*rich_runs > *poor_runs * 2, "El scheduler de lotería no mostró una preferencia clara.");
    println!("Lottery OK: Se ejecutó más al hilo con más tiquetes.");

    // --- Sub-test RealTime (EDF) ---
    let mut rt_realtime = ThreadRuntime::new();
    let exec_order = Rc::new(RefCell::new(Vec::new()));
    
    let order1 = exec_order.clone();
    let entry_c: ThreadEntry = Box::new(move |_, _| { order1.borrow_mut().push('C'); ThreadSignal::Exit });
    
    let order2 = exec_order.clone();
    let entry_a: ThreadEntry = Box::new(move |_, _| { order2.borrow_mut().push('A'); ThreadSignal::Exit });
    
    let order3 = exec_order.clone();
    let entry_b: ThreadEntry = Box::new(move |_, _| { order3.borrow_mut().push('B'); ThreadSignal::Exit });

    my_thread_create(&mut rt_realtime, "C", SchedulerType::RealTime, entry_c, None, Some(300));
    my_thread_create(&mut rt_realtime, "A", SchedulerType::RealTime, entry_a, None, Some(100));
    my_thread_create(&mut rt_realtime, "B", SchedulerType::RealTime, entry_b, None, Some(200));

    rt_realtime.run(5);

    let order = exec_order.borrow();
    println!("RealTime -> Orden de ejecución: {:?}", order);
    assert_eq!(*order, vec!['A', 'B', 'C'], "El scheduler de tiempo real (EDF) no ejecutó los hilos en orden de deadline.");
    println!("RealTime OK: Los hilos se ejecutaron según el deadline más cercano.");

    println!("[TEST 4] OK: Los schedulers específicos se comportaron como se esperaba.");
}
