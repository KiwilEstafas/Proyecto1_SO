use mypthreads::{
    my_thread_create, my_thread_end, my_thread_yield,
    my_thread_join, my_thread_detach,
    SchedulerType, ThreadRuntime, ThreadSignal
};

// uso temporal solo en las demos para proteger contadores compartidos
// esto viene de la libreria estandar y no es parte de mypthreads
// se reemplazara por el mymutex implementado en la libreria
use std::sync::{Arc, Mutex}; // NOTAAA

fn demo_round_robin() {
    println!("\n=== demo round robin ===");
    // crea runtime
    let mut rt = ThreadRuntime::new();

    // hilo a imprime 3 veces y termina
    let a_count = Arc::new(Mutex::new(0));
    let a_count_cl = Arc::clone(&a_count);
    my_thread_create(
        &mut rt,
        "a",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut v = a_count_cl.lock().unwrap();
            *v += 1;
            println!("[a] step {}", *v);
            if *v >= 3 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        None,
        None,
    );

    // hilo b imprime 2 veces y termina
    let b_count = Arc::new(Mutex::new(0));
    let b_count_cl = Arc::clone(&b_count);
    my_thread_create(
        &mut rt,
        "b",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut v = b_count_cl.lock().unwrap();
            *v += 1;
            println!("[b] step {}", *v);
            if *v >= 2 {
                return ThreadSignal::Exit;
            }
            my_thread_yield()
        }),
        None,
        None,
    );

    // bucle simple sin tiempo porque rr no lo requiere
    for _ in 0..10 {
        rt.run_once();
        if rt.now() == u64::MAX {
            break;
        }
    }

    println!("rr listo\n");
}

fn demo_lottery() {
    println!("=== demo lottery 5 2 1 tickets ===");
    let mut rt = ThreadRuntime::new();

    // hilo x con 5 tickets
    let x_count = Arc::new(Mutex::new(0));
    let x_count_cl = Arc::clone(&x_count);
    my_thread_create(
        &mut rt,
        "x",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = x_count_cl.lock().unwrap();
            *v += 1;
            if *v % 10 == 0 {
                println!("[x 5 tickets] ran {}", *v);
            }
            if *v >= 50 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(5),
        None,
    );

    // hilo y con 2 tickets
    let y_count = Arc::new(Mutex::new(0));
    let y_count_cl = Arc::clone(&y_count);
    my_thread_create(
        &mut rt,
        "y",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = y_count_cl.lock().unwrap();
            *v += 1;
            if *v >= 30 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(2),
        None,
    );

    // hilo z con 1 ticket
    let z_count = Arc::new(Mutex::new(0));
    let z_count_cl = Arc::clone(&z_count);
    my_thread_create(
        &mut rt,
        "z",
        SchedulerType::Lottery,
        Box::new(move |_rt, _tid| {
            let mut v = z_count_cl.lock().unwrap();
            *v += 1;
            if *v >= 20 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        Some(1),
        None,
    );

    // correr varios ciclos
    for _ in 0..200 {
        rt.run_once();
        if rt.now() == u64::MAX {
            break;
        }
    }

    let xv = *x_count.lock().unwrap();
    let yv = *y_count.lock().unwrap();
    let zv = *z_count.lock().unwrap();
    println!("x={}, y={}, z={}", xv, yv, zv);
    println!("esperado x > y > z por probabilidad\n");
}

fn demo_realtime_single_shot() {
    println!("=== demo realtime edf con reloj logico ===");
    let mut rt = ThreadRuntime::new();

    // hilo menos urgente con deadline absoluto 50
    my_thread_create(
        &mut rt,
        "low",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            println!("[low] executed at now={}", _rt.now());
            my_thread_end()
        }),
        None,
        Some(50),
    );

    // hilo mas urgente con deadline absoluto 10
    my_thread_create(
        &mut rt,
        "high",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            println!("[high] executed at now={}", _rt.now());
            my_thread_end()
        }),
        None,
        Some(10),
    );

    // avanzar tiempo 0 y correr una vez
    rt.advance_time(0);
    rt.run_once(); // deberia correr high primero

    // avanzar tiempo y correr otra vez
    rt.advance_time(5);
    rt.run_once(); // deberia correr low ahora

    println!("realtime single shot listo\n");
}

fn demo_realtime_periodic() {
    println!("=== demo realtime periodico simple ===");
    let mut rt = ThreadRuntime::new();

    // este ejemplo simula dos tareas con deadlines periodicos calculados fuera
    // como no tenemos period en mythread se reprograman a mano desde el bucle

    // contadores compartidos
    let t1_count = Arc::new(Mutex::new(0));
    let t2_count = Arc::new(Mutex::new(0));

    // tarea 1 con periodo 20 y primer deadline en 20
    let t1c = Arc::clone(&t1_count);
    my_thread_create(
        &mut rt,
        "t1",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            let mut c = t1c.lock().unwrap();
            *c += 1;
            println!("[t1] fired at now={} count={}", _rt.now(), *c);
            my_thread_end()
        }),
        None,
        Some(20),
    );

    // tarea 2 con periodo 30 y primer deadline en 30
    let t2c = Arc::clone(&t2_count);
    my_thread_create(
        &mut rt,
        "t2",
        SchedulerType::RealTime,
        Box::new(move |_rt, _tid| {
            let mut c = t2c.lock().unwrap();
            *c += 1;
            println!("[t2] fired at now={} count={}", _rt.now(), *c);
            my_thread_end()
        }),
        None,
        Some(30),
    );

    // bucle de 5 ticks con dt 10 ms
    // reprograma manualmente cada tarea al terminar dandole un nuevo deadline absoluto
    for step in 0..5 {
        rt.advance_time(10);
        rt.run_once();

        // si t1 termino hayq ue recrealro con deadline actual mas periodo 20
        if let Some(th) = rt.threads.get(&1) {
            if th.state == mypthreads::ThreadState::Terminated {
                let t1c2 = Arc::clone(&t1_count);
                let next_deadline = rt.now() + 20; // leer now antes de pasar &mut rt
                my_thread_create(
                    &mut rt,
                    "t1",
                    SchedulerType::RealTime,
                    Box::new(move |_rt, _tid| {
                        let mut c = t1c2.lock().unwrap();
                        *c += 1;
                        println!("[t1] fired at now={} count={}", _rt.now(), *c);
                        my_thread_end()
                    }),
                    None,
                    Some(next_deadline),
                );
            }
        }

        // si t2 termino hay que recrearlo con deadline actual mas periodo 30
        if let Some(th) = rt.threads.get(&2) {
            if th.state == mypthreads::ThreadState::Terminated {
                let t2c2 = Arc::clone(&t2_count);
                let next_deadline = rt.now() + 30; // leer now antes de pasar &mut rt
                my_thread_create(
                    &mut rt,
                    "t2",
                    SchedulerType::RealTime,
                    Box::new(move |_rt, _tid| {
                        let mut c = t2c2.lock().unwrap();
                        *c += 1;
                        println!("[t2] fired at now={} count={}", _rt.now(), *c);
                        my_thread_end()
                    }),
                    None,
                    Some(next_deadline),
                );
            }
        }

        println!("tick {} now={}", step, rt.now());
    }

    println!("realtime periodico listo\n");
}

fn demo_join_detach() {
    println!("\n=== demo join y detach ===");

    // crear runtime
    let mut rt = ThreadRuntime::new();

    // contador compartido para el worker
    let work_count = Arc::new(Mutex::new(0));
    let work_count_cl = Arc::clone(&work_count);

    // hilo worker que hace dos pasos y termina
    let worker_tid = my_thread_create(
        &mut rt,
        "worker",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut c = work_count_cl.lock().unwrap();
            *c += 1;
            println!("[worker] step {}", *c);
            if *c >= 2 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        None,
        None,
    );

    // bandera para imprimir solo una vez cuando el watcher retoma
    let resumed = Arc::new(Mutex::new(false));
    let resumed_cl = Arc::clone(&resumed);

    // hilo watcher que hace join al worker
    my_thread_create(
        &mut rt,
        "watcher",
        SchedulerType::RoundRobin,
        Box::new(move |rt, _tid| {
            // intentar join
            let sig = my_thread_join(rt, worker_tid);

            // si se bloqueo devolver block para que el runtime lo estacione
            if let ThreadSignal::Block = sig {
                println!("[watcher] esperando a worker");
                return ThreadSignal::Block;
            }

            // si no se bloqueo entonces worker ya termino
            let mut r = resumed_cl.lock().unwrap();
            if !*r {
                *r = true;
                println!("[watcher] worker termino y watcher retoma");
            }
            my_thread_end()
        }),
        None,
        None,
    );

    // crear un hilo corto que vamos a marcar como detached antes de correr
    let short_count = Arc::new(Mutex::new(0));
    let short_count_cl = Arc::clone(&short_count);
    let shorty_tid = my_thread_create(
        &mut rt,
        "shorty",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut c = short_count_cl.lock().unwrap();
            *c += 1;
            println!("[shorty] step {}", *c);
            if *c >= 1 {
                return my_thread_end();
            }
            my_thread_yield()
        }),
        None,
        None,
    );

    // marcar shorty como detached antes de ejecutar
    my_thread_detach(&mut rt, shorty_tid);
    println!("[main] shorty marcado como detached tid={}", shorty_tid);

    // correr suficientes ciclos para que worker termine y watcher retome
    rt.run(100);

    // verificar que shorty fue eliminado del mapa de hilos al terminar por estar detached
    let exists = rt.threads.contains_key(&shorty_tid);
    println!("[main] shorty sigue presente en tabla de hilos = {}", exists);
    println!("demo join y detach lista\n");
}


fn main() {
    println!("threadcity schedulers demo");

    demo_round_robin();
     demo_lottery();
    demo_realtime_single_shot();
    demo_realtime_periodic();

    demo_join_detach();

    println!("todas las demos completadas");
}


