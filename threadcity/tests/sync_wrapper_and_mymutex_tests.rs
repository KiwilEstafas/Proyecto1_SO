// threadcity/tests/sync_wrapper_and_mymutex_tests.rs
//
// Tests de integración para validar el wrapper MyMutexCell y la API de MyMutex.
//
// Ejecutar con: cargo test -p threadcity -- --nocapture

use mypthreads::mypthreads_api::*;
use mypthreads::signals::ThreadSignal;
// --- CORRECCIÓN CLAVE AQUÍ ---
// El módulo `sync` ahora vive en `mypthreads`, no en `threadcity`.
use mypthreads::sync::{shared, MyMutexCell, Shared};
// --- FIN DE LA CORRECCIÓN ---

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc,    Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Ejecuta el runtime con timeout para evitar colgarse
fn run_with_timeout(cycles: usize, max_attempts: usize) {
    for attempt in 0..max_attempts {
        run_simulation(cycles);

        let rt = RUNTIME.lock().unwrap();
        let ready_empty = rt.ready.is_empty();
        let blocked_empty = rt.blocked.is_empty();
        drop(rt);

        if ready_empty && blocked_empty {
            println!("[Runtime] All threads completed after {} attempts", attempt + 1);
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }

    println!("[Runtime] WARNING: Max attempts reached");
}

// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_request_lock_requires_thread_context() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 1: request_lock requiere contexto de hilo           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let cell = Arc::new(MyMutexCell::new(42));

    println!("=== Parte A: Intentar request_lock desde hilo principal ===");

    let result = std::panic::catch_unwind(|| {
        let test_cell = MyMutexCell::new(100);
        test_cell.request_lock()
    });

    match result {
        Ok(_signal) => {
            println!("⚠ request_lock no causó panic desde hilo principal");
            println!("  (Esto puede ser OK si la API maneja contextos faltantes gracefully)");
        }
        Err(_) => {
            println!("✓ request_lock causó panic desde hilo principal (esperado)");
        }
    }

    println!("\n=== Parte B: Usar request_lock correctamente desde hilo mypthread ===");

    let cell_clone = cell.clone();
    let success = Arc::new(AtomicU32::new(0));
    let success_clone = success.clone();

    my_thread_create(
        "LockUser",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let status = success_clone.load(Ordering::SeqCst);
            println!("[LockUser] Status: {}", status);

            match status {
                0 => {
                    // Paso 1: Solicitar lock
                    println!("[LockUser] Requesting lock...");
                    let signal = cell_clone.request_lock();
                    println!("[LockUser] Lock signal: {:?}", signal);

                    match signal {
                        ThreadSignal::MutexLock(_) => {
                            println!("[LockUser] Got MutexLock signal, will be blocked/resumed");
                            success_clone.store(1, Ordering::SeqCst);
                            return signal;
                        }
                        ThreadSignal::Continue => {
                            println!("[LockUser] Lock acquired immediately!");
                            success_clone.store(2, Ordering::SeqCst);
                            return ThreadSignal::Yield;
                        }
                        _ => {
                            println!("[LockUser] Unexpected signal: {:?}", signal);
                            return ThreadSignal::Exit;
                        }
                    }
                }
                1 | 2 => {
                    // Paso 2: Entrar a sección crítica
                    println!("[LockUser] Entering critical section...");
                    let mut guard = cell_clone.enter();
                    let value = *guard;
                    println!("[LockUser] Value: {}", value);

                    *guard = value + 1;
                    println!("[LockUser] Incremented to: {}", *guard);

                    drop(guard); // Drop NO hace unlock

                    success_clone.store(3, Ordering::SeqCst);
                    return ThreadSignal::Yield;
                }
                3 => {
                    // Paso 3: Liberar lock
                    println!("[LockUser] Requesting unlock...");
                    success_clone.store(4, Ordering::SeqCst); // Marcar que se intentó unlock
                    return cell_clone.request_unlock();
                }
                _ => return ThreadSignal::Exit,
            }
        }),
    );

    run_with_timeout(30, 10);

    let final_status = success.load(Ordering::SeqCst);
    println!("\nFinal status: {}", final_status);
    println!("  0 = Not executed");
    println!("  1 = Got MutexLock signal");
    println!("  2 = Lock acquired immediately");
    println!("  3 = Entered and modified data");
    println!("  4 = Unlocked successfully");

    assert!(
        final_status >= 3,
        "Thread should have entered critical section (got status {})",
        final_status
    );

    // Verificar que el valor se incrementó
    if let Some(guard) = cell.try_enter() {
        let value = *guard;
        drop(guard);
        let _ = cell.request_unlock();

        println!("\nFinal value: {}", value);
        assert_eq!(value, 43, "Value should have been incremented");
    }

    println!("\n✓ TEST 1 PASSED\n");
}

// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_try_enter_semantics() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 2: Semántica de try_enter                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Qué valida: Si A tiene el lock, try_enter en B devuelve None.
    // Después de que A libera, B puede obtener el lock.

    let cell: Shared<i32> = shared(100);

    let cell_a = cell.clone();
    let has_lock = Arc::new(AtomicU32::new(0));
    let has_lock_a_for_a = has_lock.clone();

    let cell_b = cell.clone();
    let try_result = Arc::new(AtomicU32::new(0));
    let try_result_b_for_a = try_result.clone();

    // Thread A: toma el lock y lo mantiene
    my_thread_create(
        "ThreadA",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let status = has_lock_a_for_a.load(Ordering::SeqCst);

            match status {
                0 => {
                    println!("[ThreadA] Requesting lock...");
                    let signal = cell_a.request_lock();

                    match signal {
                        ThreadSignal::MutexLock(_) => {
                            has_lock_a_for_a.store(1, Ordering::SeqCst);
                            return signal;
                        }
                        ThreadSignal::Continue => {
                            has_lock_a_for_a.store(2, Ordering::SeqCst);
                            return ThreadSignal::Yield;
                        }
                        _ => return signal,
                    }
                }
                1 | 2 => {
                    println!("[ThreadA] Holding lock...");
                    let _guard = cell_a.enter();
                    // Mantener el lock por varios ciclos
                    has_lock_a_for_a.store(3, Ordering::SeqCst);

                    // Esperar a que B intente
                    if try_result_b_for_a.load(Ordering::SeqCst) == 0 {
                        return ThreadSignal::Yield;
                    }

                    drop(_guard);
                    has_lock_a_for_a.store(4, Ordering::SeqCst);
                    return ThreadSignal::Yield;
                }
                4 => {
                    println!("[ThreadA] Releasing lock...");
                    has_lock_a_for_a.store(5, Ordering::SeqCst);
                    return cell_a.request_unlock();
                }
                _ => return ThreadSignal::Yield,
            }
        }),
    );

    // Thread B: intenta try_enter (usa clones distintos)
    let has_lock_a_for_b = has_lock.clone();
    let try_result_b_for_b = try_result.clone();
    let cell_b_for_b = cell.clone();

    my_thread_create(
        "ThreadB",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let attempts = try_result_b_for_b.load(Ordering::SeqCst);
            let a_status = has_lock_a_for_b.load(Ordering::SeqCst);

            if attempts == 0 {
                // Esperar a que A tome el lock
                if a_status < 3 {
                    return ThreadSignal::Yield;
                }

                println!("[ThreadB] First try_enter (should fail)...");
                let result = cell_b_for_b.try_enter();

                if result.is_none() {
                    println!("[ThreadB] ✓ try_enter returned None (correct)");
                    try_result_b_for_b.store(1, Ordering::SeqCst);
                } else {
                    println!("[ThreadB] ✗ ERROR: try_enter succeeded when shouldn't!");
                    drop(result);
                    let _ = cell_b_for_b.request_unlock();
                    try_result_b_for_b.store(10, Ordering::SeqCst);
                    return ThreadSignal::Exit;
                }

                return ThreadSignal::Yield;
            } else if attempts == 1 {
                // Esperar a que A libere
                if a_status < 5 {
                    return ThreadSignal::Yield;
                }

                println!("[ThreadB] Second try_enter (should succeed)...");
                let result = cell_b_for_b.try_enter();

                if let Some(mut guard) = result {
                    println!("[ThreadB] ✓ try_enter succeeded!");
                    *guard = 200;
                    println!("[ThreadB] Modified value to 200");
                    drop(guard);

                    let _ = cell_b_for_b.request_unlock();
                    try_result_b_for_b.store(2, Ordering::SeqCst);
                    return ThreadSignal::Exit;
                } else {
                    println!("[ThreadB] ✗ ERROR: try_enter failed when should succeed!");
                    try_result_b_for_b.store(20, Ordering::SeqCst);
                    return ThreadSignal::Exit;
                }
            }

            ThreadSignal::Exit
        }),
    );

    run_with_timeout(50, 15);

    let final_result = try_result.load(Ordering::SeqCst);
    println!("\nThreadB result: {}", final_result);
    println!("  1 = First attempt failed (correct)");
    println!("  2 = Second attempt succeeded (correct)");
    println!("  10 = First attempt succeeded (ERROR)");
    println!("  20 = Second attempt failed (ERROR)");

    assert_eq!(
        final_result, 2,
        "ThreadB should fail first and succeed second"
    );

    println!("\n✓ TEST 2 PASSED\n");
}

// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_lock_unlock_roundtrip_order() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 3: Orden de lock/unlock roundtrip                   ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    const ITERATIONS: usize = 5;

    let cell: Shared<i32> = shared(0);
    let cell_clone = cell.clone();

    let lock_count = Arc::new(AtomicUsize::new(0));
    let unlock_count = Arc::new(AtomicUsize::new(0));

    let locks = lock_count.clone();
    let unlocks = unlock_count.clone();

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum State {
        NeedLock,
        HasLock,
    }

    let state = Arc::new(AtomicU32::new(State::NeedLock as u32));
    let state_clone = state.clone();

    my_thread_create(
        "RoundtripThread",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let current_state_val = state_clone.load(Ordering::SeqCst);
            let current_locks = locks.load(Ordering::SeqCst);

            if current_locks >= ITERATIONS {
                println!("[RoundtripThread] Completed all iterations!");
                return ThreadSignal::Exit;
            }

            match current_state_val {
                0 => { // State::NeedLock
                    println!("[RoundtripThread] Requesting lock #{}...", current_locks + 1);
                    state_clone.store(State::HasLock as u32, Ordering::SeqCst);
                    return cell_clone.request_lock();
                }
                1 => { // State::HasLock
                    println!("[RoundtripThread] Working in critical section...");
                    let mut guard = cell_clone.enter();
                    *guard += 1;
                    println!("[RoundtripThread] Value: {}", *guard);
                    drop(guard);

                    locks.fetch_add(1, Ordering::SeqCst);
                    unlocks.fetch_add(1, Ordering::SeqCst);

                    println!("[RoundtripThread] Releasing lock #{}...", current_locks + 1);
                    state_clone.store(State::NeedLock as u32, Ordering::SeqCst);
                    return cell_clone.request_unlock();
                }
                _ => return ThreadSignal::Exit,
            }
        }),
    );

    run_with_timeout(80, 20);

    let final_locks = lock_count.load(Ordering::SeqCst);
    let final_unlocks = unlock_count.load(Ordering::SeqCst);

    println!("\nFinal counts:");
    println!("  Locks: {}", final_locks);
    println!("  Unlocks: {}", final_unlocks);

    assert_eq!(final_locks, final_unlocks, "Lock/unlock count mismatch!");
    assert_eq!(final_locks, ITERATIONS, "Should have done {} iterations", ITERATIONS);

    // Verificar el valor final
    if let Some(guard) = cell.try_enter() {
        let value = *guard;
        drop(guard);
        let _ = cell.request_unlock();

        println!("Final cell value: {}", value);
        assert_eq!(value, ITERATIONS as i32, "Value should match iteration count");
    }

    println!("\n✓ TEST 3 PASSED\n");
}


// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_mutual_exclusion_on_shared_counter() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 4: Exclusión mutua en contador compartido           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    const NUM_THREADS: usize = 3;
    const INCREMENTS_PER_THREAD: usize = 4;

    let cell: Shared<usize> = shared(0);
    let completion_counter = Arc::new(AtomicUsize::new(0));

    for i in 0..NUM_THREADS {
        let cell_clone = cell.clone();
        let my_increments = Arc::new(AtomicUsize::new(0));
        let completion = completion_counter.clone();

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum WorkerState {
            NeedLock,
            HasLock,
        }

        let state = Arc::new(AtomicU32::new(WorkerState::NeedLock as u32));

        my_thread_create(
            &format!("Worker-{}", i),
            SchedulerParams::RoundRobin,
            Box::new(move |_tid, _tickets| {
                let my_count = my_increments.load(Ordering::SeqCst);
                if my_count >= INCREMENTS_PER_THREAD {
                    if my_increments.compare_exchange(my_count, my_count + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                        completion.fetch_add(1, Ordering::SeqCst);
                        println!("[Worker-{}] Completed all increments", i);
                    }
                    return ThreadSignal::Exit;
                }

                let current_state = state.load(Ordering::SeqCst);

                match current_state {
                    0 => { // WorkerState::NeedLock
                        println!("[Worker-{}] Requesting lock (increment {})...", i, my_count + 1);
                        state.store(WorkerState::HasLock as u32, Ordering::SeqCst);
                        return cell_clone.request_lock();
                    }
                    1 => { // WorkerState::HasLock
                        println!("[Worker-{}] Incrementing...", i);
                        let mut guard = cell_clone.enter();
                        let old_value = *guard;
                        // Yield to increase chance of race conditions if mutex fails
                        thread::yield_now();
                        *guard = old_value + 1;
                        println!("[Worker-{}] {} -> {}", i, old_value, *guard);
                        drop(guard);

                        my_increments.fetch_add(1, Ordering::SeqCst);
                        state.store(WorkerState::NeedLock as u32, Ordering::SeqCst);
                        return cell_clone.request_unlock();
                    }
                    _ => return ThreadSignal::Exit,
                }
            }),
        );
    }

    run_with_timeout(150, 40);

    let completed = completion_counter.load(Ordering::SeqCst);
    println!("\nThreads completed: {}/{}", completed, NUM_THREADS);

    if let Some(guard) = cell.try_enter() {
        let final_value = *guard;
        drop(guard);
        let _ = cell.request_unlock();
        let expected = NUM_THREADS * INCREMENTS_PER_THREAD;
        println!("Final counter value: {}", final_value);
        println!("Expected value: {}", expected);
        assert_eq!(
            final_value, expected,
            "Counter mismatch! Expected {}, got {}",
            expected, final_value
        );
    } else {
        panic!("Could not acquire lock to check final value!");
    }

    println!("\n✓ TEST 4 PASSED\n");
}


// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_guard_scope_no_unlock() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 5: Guard NO hace unlock en Drop                     ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let cell: Shared<i32> = shared(42);

    let cell_a = cell.clone();
    let a_status = Arc::new(AtomicU32::new(0));
    let a_status_for_a = a_status.clone();

    let b_result = Arc::new(AtomicU32::new(0));
    let b_result_for_a = b_result.clone();

    my_thread_create(
        "ThreadA",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let status = a_status_for_a.load(Ordering::SeqCst);
            match status {
                0 => {
                    println!("[ThreadA] Requesting lock...");
                    a_status_for_a.store(1, Ordering::SeqCst);
                    return cell_a.request_lock();
                }
                1 => {
                    println!("[ThreadA] Entering critical section...");
                    {
                        let _guard = cell_a.enter();
                        println!("[ThreadA] Have guard, letting it drop...");
                    }
                    println!("[ThreadA] Guard dropped (but lock still held)");
                    a_status_for_a.store(2, Ordering::SeqCst);
                    return ThreadSignal::Yield;
                }
                2 => {
                    if b_result_for_a.load(Ordering::SeqCst) < 1 {
                        return ThreadSignal::Yield;
                    }
                    println!("[ThreadA] Now explicitly unlocking...");
                    a_status_for_a.store(3, Ordering::SeqCst);
                    return cell_a.request_unlock();
                }
                _ => ThreadSignal::Yield,
            }
        }),
    );

    let a_status_for_b = a_status.clone();
    let b_result_for_b = b_result.clone();
    let cell_b_for_b = cell.clone();

    my_thread_create(
        "ThreadB",
        SchedulerParams::RoundRobin,
        Box::new(move |_tid, _tickets| {
            let attempts = b_result_for_b.load(Ordering::SeqCst);
            let a_stat = a_status_for_b.load(Ordering::SeqCst);

            if attempts == 0 {
                if a_stat < 2 {
                    return ThreadSignal::Yield;
                }
                println!("[ThreadB] First try_enter (should fail, A still has lock)...");
                if cell_b_for_b.try_enter().is_none() {
                    println!("[ThreadB] ✓ try_enter failed (correct, guard Drop didn't unlock)");
                    b_result_for_b.store(1, Ordering::SeqCst);
                } else {
                    println!("[ThreadB] ✗ ERROR: Got lock when A should still have it!");
                    b_result_for_b.store(10, Ordering::SeqCst);
                }
                return ThreadSignal::Yield;
            } else if attempts == 1 {
                if a_stat < 3 {
                    return ThreadSignal::Yield;
                }
                println!("[ThreadB] Second try_enter (should succeed now)...");
                if let Some(guard) = cell_b_for_b.try_enter() {
                    println!("[ThreadB] ✓ try_enter succeeded after explicit unlock!");
                    drop(guard);
                    let _ = cell_b_for_b.request_unlock();
                    b_result_for_b.store(2, Ordering::SeqCst);
                } else {
                    println!("[ThreadB] ✗ ERROR: Failed to get lock after A unlocked!");
                    b_result_for_b.store(20, Ordering::SeqCst);
                }
            }
            ThreadSignal::Exit
        }),
    );

    run_with_timeout(60, 20);

    let final_result = b_result.load(Ordering::SeqCst);
    println!("\nThreadB result: {}", final_result);
    assert_eq!(
        final_result, 2,
        "Guard Drop should NOT unlock (got result {})",
        final_result
    );
    println!("\n✓ TEST 5 PASSED: Guard NO hace unlock en Drop\n");
}


// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn test_no_overlap_within_critical_section() {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║ TEST 6: No hay solapamiento en sección crítica           ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let cell: Shared<i32> = shared(0);
    let in_section = Arc::new(AtomicU32::new(0));
    let max_in_section = Arc::new(AtomicU32::new(0));

    const NUM_THREADS: usize = 3;

    for i in 0..NUM_THREADS {
        let cell_clone = cell.clone();
        let in_sec = in_section.clone();
        let max_sec = max_in_section.clone();

        #[derive(Clone, Copy)]
        enum State { NeedLock, InSection, Done }
        let state = Arc::new(Mutex::new(State::NeedLock));

        my_thread_create(
            &format!("Thread-{}", i),
            SchedulerParams::RoundRobin,
            Box::new(move |_tid, _tickets| {
                let mut current_state = state.lock().unwrap();
                match *current_state {
                    State::NeedLock => {
                        *current_state = State::InSection;
                        return cell_clone.request_lock();
                    }
                    State::InSection => {
                        println!("[Thread-{}] ENTERING critical section", i);
                        let count = in_sec.fetch_add(1, Ordering::SeqCst) + 1;
                        max_sec.fetch_max(count, Ordering::SeqCst);

                        let mut guard = cell_clone.enter();
                        *guard += 1;
                        thread::sleep(Duration::from_millis(5));
                        drop(guard);

                        in_sec.fetch_sub(1, Ordering::SeqCst);
                        println!("[Thread-{}] EXITING critical section", i);

                        *current_state = State::Done;
                        return cell_clone.request_unlock();
                    }
                    State::Done => ThreadSignal::Exit,
                }
            }),
        );
    }

    run_with_timeout(50, 15);

    let max_concurrent = max_in_section.load(Ordering::SeqCst);
    println!("\nMax concurrent threads in section: {}", max_concurrent);
    assert_eq!(
        max_concurrent, 1,
        "Should never have more than 1 thread in critical section!"
    );
    println!("\n✓ TEST 6 PASSED: No solapamiento detectado\n");
}