use mypthreads::{
    my_thread_create, my_thread_end, my_thread_yield, SchedulerType, ThreadRuntime, ThreadSignal,
};

fn main() {
    println!("ThreadCity demo: Round Robin MVP");

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
    );

    // Corre el "scheduler" por unos ciclos
    for _ in 0..10 {
        rt.run_once();
    }

    println!("Done.");
}


