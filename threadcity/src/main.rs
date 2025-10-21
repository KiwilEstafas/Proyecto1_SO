use mypthreads::{
    my_thread_create, my_thread_end, my_thread_yield, SchedulerType, ThreadRuntime, ThreadSignal,
};

fn demo_round_robin() {
    println!("\n=== ThreadCity Demo: Round Robin ===");
    let mut rt = ThreadRuntime::new();

    // Hilo A: imprime 3 veces y termina
    let mut a_count = 0;
    my_thread_create(
        &mut rt,
        "A",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            a_count += 1;
            println!("[A] step {a_count}");
            if a_count >= 3 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        None,
    );

    // Hilo B: imprime 2 veces y termina
    let mut b_count = 0;
    my_thread_create(
        &mut rt,
        "B",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            b_count += 1;
            println!("[B] step {b_count}");
            if b_count >= 2 {
                return ThreadSignal::Exit;
            }
            my_thread_yield()
        }),
        None,
    );

    // Corre el "scheduler" por unos ciclos
    for _ in 0..10 {
        rt.run_once();
    }
    println!("Round Robin Done.\n");
}

fn demo_lottery() {
    println!("=== ThreadCity Demo: Lottery Scheduler ===");
    let mut rt = ThreadRuntime::new();

    // Hilo X: 5 tickets (alta prioridad)
    let mut x_count = 0;
    my_thread_create(
        &mut rt,
        "X",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            x_count += 1;
            println!("[X-5tickets] step {x_count}");
            if x_count >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(5), // 5 tickets
    );

    // Hilo Y: 2 tickets (prioridad media)
    let mut y_count = 0;
    my_thread_create(
        &mut rt,
        "Y",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            y_count += 1;
            println!("[Y-2tickets] step {y_count}");
            if y_count >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(2), // 2 tickets
    );

    // Hilo Z: 1 ticket (baja prioridad)
    let mut z_count = 0;
    my_thread_create(
        &mut rt,
        "Z",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            z_count += 1;
            println!("[Z-1ticket] step {z_count}");
            if z_count >= 10 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(1), // 1 ticket
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
    let mut fast_count = 0;
    my_thread_create(
        &mut rt,
        "FAST",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            fast_count += 1;
            if fast_count % 10 == 0 {
                println!("[FAST-90tickets] ejecutado {} veces", fast_count);
            }
            if fast_count >= 50 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(90), // 90 tickets
    );

    // Hilo SLOW: 10 tickets
    let mut slow_count = 0;
    my_thread_create(
        &mut rt,
        "SLOW",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            slow_count += 1;
            println!("[SLOW-10tickets] ejecutado {} veces", slow_count);
            if slow_count >= 50 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(10), // 10 tickets
    );

    rt.run(200);
    
    println!("Lottery Extreme Done.\n");
}

fn main() {
    println!("║   ThreadCity Schedulers Demo      ║");

    // Demostración 1: Round Robin (reparto equitativo)
    demo_round_robin();

    // Demostración 2: Lottery con 3 hilos (5:2:1 tickets)
    demo_lottery();

    // Demostración 3: Lottery extremo (90:10 tickets)
    demo_lottery_extreme();

    println!("✓ Todas las demos completadas!");
}