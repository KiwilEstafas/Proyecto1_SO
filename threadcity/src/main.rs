use std::sync::{Arc, Mutex};
use mypthreads::{
    my_thread_create, my_thread_end, my_thread_yield, SchedulerType, ThreadRuntime, ThreadSignal,
};

fn demo_round_robin() {
    println!("\n=== ThreadCity Demo: Round Robin ===");
    let mut rt = ThreadRuntime::new();

    // Hilo A: imprime 3 veces y termina
    let a_count = Arc::new(Mutex::new(0));
    let a_count_cl = Arc::clone(&a_count);
    my_thread_create(
        &mut rt,
        "A",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut v = a_count_cl.lock().unwrap();
            *v += 1;
            println!("[A] step {}", *v);
            if *v >= 3 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        None, // tickets
        None, // deadline
    );

    // Hilo B: imprime 2 veces y termina
    let b_count = Arc::new(Mutex::new(0));
    let b_count_cl = Arc::clone(&b_count);
    my_thread_create(
        &mut rt,
        "B",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut v = b_count_cl.lock().unwrap();
            *v += 1;
            println!("[B] step {}", *v);
            if *v >= 2 {
                return ThreadSignal::Exit;
            }
            my_thread_yield()
        }),
        None,
        None,
    );

    // Corre el "scheduler" por unos ciclos
    rt.run(10);
    println!("Round Robin Done.\n");
}

fn demo_lottery() {
    println!("=== ThreadCity Demo: Lottery Scheduler ===");
    let mut rt = ThreadRuntime::new();

    // Hilo X: 5 tickets (alta prioridad)
    let x_count = Arc::new(Mutex::new(0));
    let x_count_cl = Arc::clone(&x_count);
    my_thread_create(
        &mut rt,
        "X",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = x_count_cl.lock().unwrap();
            *v += 1;
            println!("[X-5tickets] step {}", *v);
            if *v >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(5), // tickets
        None,    // deadline
    );

    // Hilo Y: 2 tickets (prioridad media)
    let y_count = Arc::new(Mutex::new(0));
    let y_count_cl = Arc::clone(&y_count);
    my_thread_create(
        &mut rt,
        "Y",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = y_count_cl.lock().unwrap();
            *v += 1;
            println!("[Y-2tickets] step {}", *v);
            if *v >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(2),
        None,
    );

    // Hilo Z: 1 ticket (baja prioridad)
    let z_count = Arc::new(Mutex::new(0));
    let z_count_cl = Arc::clone(&z_count);
    my_thread_create(
        &mut rt,
        "Z",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = z_count_cl.lock().unwrap();
            *v += 1;
            println!("[Z-1ticket] step {}", *v);
            if *v >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(1),
        None,
    );

    // Ejecutar muchos ciclos para ver la distribución probabilística
    rt.run(100);

    println!("Lottery Done.\n");
    println!("Nota: X debería ejecutarse ~5/8 del tiempo, Y ~2/8, Z ~1/8");
}

fn demo_lottery_extreme() {
    println!("=== ThreadCity Demo: Lottery Extreme (90% vs 10%) ===");
    let mut rt = ThreadRuntime::new();

    // Hilo FAST: 90 tickets
    let fast_count = Arc::new(Mutex::new(0));
    let fast_count_cl = Arc::clone(&fast_count);
    my_thread_create(
        &mut rt,
        "FAST",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = fast_count_cl.lock().unwrap();
            *v += 1;
            if *v % 10 == 0 {
                println!("[FAST-90tickets] ejecutado {} veces", *v);
            }
            if *v >= 50 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(90), // tickets
        None,
    );

    // Hilo SLOW: 10 tickets
    let slow_count = Arc::new(Mutex::new(0));
    let slow_count_cl = Arc::clone(&slow_count);
    my_thread_create(
        &mut rt,
        "SLOW",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = slow_count_cl.lock().unwrap();
            *v += 1;
            println!("[SLOW-10tickets] ejecutado {} veces", *v);
            if *v >= 50 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(10), // tickets
        None,
    );

    rt.run(200);

    println!("Lottery Extreme Done.\n");
}

fn demo_realtime() {
    println!("=== ThreadCity Demo: RealTime (Earliest Deadline First) ===");
    let mut rt = ThreadRuntime::new();

    // Hilo con deadline más lejano (menos urgente)
    my_thread_create(
        &mut rt,
        "LowRT",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            println!("[LowRT] executed");
            my_thread_end()
        }),
        None,
        Some(50), // deadline (ticks/logical)
    );

    // Hilo con deadline más cercano (más urgente)
    my_thread_create(
        &mut rt,
        "HighRT",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            println!("[HighRT] executed");
            my_thread_end()
        }),
        None,
        Some(10),
    );

    // Ejecutar suficientes ciclos para observar orden
    rt.run(4);

    println!("RealTime demo Done.\n");
}

fn main() {
    println!("║   ThreadCity Schedulers Demo      ║");

    // Demostración 1: Round Robin (reparto equitativo)
    demo_round_robin();

    // Demostración 2: Lottery con 3 hilos (5:2:1 tickets)
    demo_lottery();

    // Demostración 3: Lottery extremo (90:10 tickets)
    demo_lottery_extreme();

    // Demostración 4: RealTime EDF demo
    demo_realtime();

    println!("✓ Todas las demos completadas!");
}
