use mypthreads::{
    my_thread_create, my_thread_end, my_thread_yield,
    my_thread_join, my_thread_detach,
    SchedulerType, ThreadRuntime, ThreadSignal
};

// uso temporal solo en las demos para proteger contadores compartidos
// esto viene de la libreria estandar y no es parte de mypthreads
// se reemplazara por el mymutex implementado en la libreria
use std::sync::{Arc, Mutex};

// capa ffi con firmas tipo pthreads
use std::ffi::c_void;
use mypthreads::ffi as myffi;

// rutina de ejemplo con firma c para la capa ffi
extern "C" fn c_worker(arg: *mut c_void) -> *mut c_void {
    println!("[c worker] start arg={:?}", arg);
    arg
}

fn demo_round_robin() {
    println!("\n=== demo round robin ===");
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
            if *v >= 3 { return my_thread_end(); }
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
            if *v >= 2 { return ThreadSignal::Exit; }
            my_thread_yield()
        }),
        None,
        None,
    );

    // bucle simple sin tiempo porque rr no lo requiere
    for _ in 0..10 {
        rt.run_once();
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
            if *v % 10 == 0 { println!("[x 5 tickets] ran {}", *v); }
            if *v >= 50 { return my_thread_end(); }
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
            if *v >= 30 { return my_thread_end(); }
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
            if *v >= 20 { return my_thread_end(); }
            my_thread_yield()
        }),
        Some(1),
        None,
    );

    for _ in 0..200 {
        rt.run_once();
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

    rt.advance_time(0);
    rt.run_once(); // deberia correr high primero

    rt.advance_time(5);
    rt.run_once(); // deberia correr low ahora

    println!("realtime single shot listo\n");
}

fn demo_realtime_periodic() {
    println!("=== demo realtime periodico simple ===");
    let mut rt = ThreadRuntime::new();

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
    for step in 0..5 {
        rt.advance_time(10);
        rt.run_once();

        // si t1 termino recrearlo con deadline actual + 20
        if let Some(th) = rt.threads.get(&1) {
            if th.state == mypthreads::ThreadState::Terminated {
                let t1c2 = Arc::clone(&t1_count);
                let next_deadline = rt.now() + 20;
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

        // si t2 termino recrearlo con deadline actual + 30
        if let Some(th) = rt.threads.get(&2) {
            if th.state == mypthreads::ThreadState::Terminated {
                let t2c2 = Arc::clone(&t2_count);
                let next_deadline = rt.now() + 30;
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
    let mut rt = ThreadRuntime::new();

    let work_count = Arc::new(Mutex::new(0));
    let work_count_cl = Arc::clone(&work_count);

    // worker hace dos pasos y termina
    let worker_tid = my_thread_create(
        &mut rt,
        "worker",
        SchedulerType::RoundRobin,
        Box::new(move |_rt, _tid| {
            let mut c = work_count_cl.lock().unwrap();
            *c += 1;
            println!("[worker] step {}", *c);
            if *c >= 2 { return my_thread_end(); }
            my_thread_yield()
        }),
        None,
        None,
    );

    // watcher que espera al worker
    let resumed = Arc::new(Mutex::new(false));
    let resumed_cl = Arc::clone(&resumed);
    my_thread_create(
        &mut rt,
        "watcher",
        SchedulerType::RoundRobin,
        Box::new(move |rt, _tid| {
            let sig = my_thread_join(rt, worker_tid);
            if let ThreadSignal::Block = sig {
                println!("[watcher] esperando a worker");
                return ThreadSignal::Block;
            }
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

    // hilo corto que marcamos detached
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
            if *c >= 1 { return my_thread_end(); }
            my_thread_yield()
        }),
        None,
        None,
    );

    my_thread_detach(&mut rt, shorty_tid);
    println!("[main] shorty marcado como detached tid={}", shorty_tid);

    rt.run(100);

    let exists = rt.threads.contains_key(&shorty_tid);
    println!("[main] shorty sigue presente en tabla de hilos = {}", exists);
    println!("demo join y detach lista\n");
}

// demo que usa la capa ffi con firmas estilo pthreads
fn demo_pthreads_facade() {
    use std::mem::MaybeUninit;
    use std::ptr;

    println!("\n=== demo capa ffi estilo pthreads ===");

    // crear hilo y esperar su retorno void*
    let mut tid: myffi::my_thread_t = 0;
    let rc_create = myffi::my_thread_create(
        &mut tid as *mut myffi::my_thread_t,
        ptr::null(),
        c_worker,
        123 as *mut c_void,
    );
    println!("[ffi] create rc={} tid={}", rc_create, tid);

    let mut retval: *mut c_void = ptr::null_mut();
    let rc_join = myffi::my_thread_join(tid, &mut retval as *mut *mut c_void);
    println!("[ffi] join rc={} retval={:?}", rc_join, retval);

    let mut m = MaybeUninit::<myffi::my_mutex_t>::uninit();
    let rc_mi = myffi::my_mutex_init(m.as_mut_ptr(), ptr::null());
    assert_eq!(rc_mi, 0, "mutex init fallo");

    // ya inicializado, es seguro asumir init
    let mut m = unsafe { m.assume_init() };
    let rc_md = myffi::my_mutex_destroy(&mut m as *mut myffi::my_mutex_t);
    println!("[ffi] mutex init rc={} destroy rc={}", rc_mi, rc_md);

    // detach sobre un tid inexistente devuelve error no cero
    let rc_det = myffi::my_thread_detach(9999);
    println!("[ffi] detach rc={}", rc_det);

    println!("demo ffi lista\n");
}

fn main() {
    println!("threadcity schedulers demo");

    demo_round_robin();
    demo_lottery();
    demo_realtime_single_shot();
    demo_realtime_periodic();
    demo_join_detach();

    // demo extra para verificar la capa de firmas pthread
    demo_pthreads_facade();

    println!("todas las demos completadas");
}

