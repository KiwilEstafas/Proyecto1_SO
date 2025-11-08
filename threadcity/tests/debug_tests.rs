// Test exhaustivo para identificar problemas en ThreadCity

use threadcity::*;
use mypthreads::mypthreads_api::*;
use mypthreads::signals::ThreadSignal;
use std::sync::{Arc, Mutex};

#[test]
fn test_01_minimal_thread() {
    println!("\n=== TEST 1: Hilo mínimo (solo Exit) ===");
    
    my_thread_create(
        "Minimal",
        SchedulerParams::RoundRobin,
        Box::new(|tid| {
            println!("  [Hilo {}] Ejecutando", tid);
            ThreadSignal::Exit
        }),
    );
    
    println!("  Ejecutando 3 ciclos...");
    run_simulation(3);
    println!("  ✓ Test 1 pasó\n");
}

#[test]
fn test_02_thread_with_yield() {
    println!("\n=== TEST 2: Hilo con Yield ===");
    
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    
    my_thread_create(
        "Yielder",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            let current = *count;
            drop(count);
            
            println!("  [Hilo {}] Ejecución #{}", tid, current);
            
            if current < 3 {
                ThreadSignal::Yield
            } else {
                println!("  [Hilo {}] Terminando", tid);
                ThreadSignal::Exit
            }
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    
    let final_count = *counter.lock().unwrap();
    assert_eq!(final_count, 3);
    println!("  ✓ Test 2 pasó (ejecutó {} veces)\n", final_count);
}

#[test]
fn test_03_create_city_only() {
    println!("\n=== TEST 3: Solo crear ciudad (sin hilos) ===");
    
    let (city, layout) = create_city();
    
    println!("  Ciudad creada:");
    println!("    Grid: {}x{}", layout.grid_rows, layout.grid_cols);
    println!("    Puentes: {}", city.bridges.len());
    println!("    Plantas: {}", city.plants.len());
    
    assert_eq!(city.bridges.len(), 3);
    assert_eq!(city.plants.len(), 2);
    
    println!("  ✓ Test 3 pasó\n");
}

#[test]
fn test_04_shared_city_access() {
    println!("\n=== TEST 4: Acceso a ciudad compartida ===");
    
    let (city, _layout) = create_city();
    let shared_city = create_shared_city(city);
    
    // Clonar para el hilo
    let city_clone = Arc::clone(&shared_city);
    
    my_thread_create(
        "CityReader",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            println!("  [Hilo {}] Intentando acceder a ciudad...", tid);
            
            let city_lock = city_clone.lock().unwrap();
            let time = city_lock.current_time();
            let bridges = city_lock.bridges.len();
            drop(city_lock);
            
            println!("  [Hilo {}] Ciudad accedida OK (time: {}, bridges: {})", tid, time, bridges);
            ThreadSignal::Exit
        }),
    );
    
    println!("  Ejecutando 3 ciclos...");
    run_simulation(3);
    println!("  ✓ Test 4 pasó\n");
}

#[test]
fn test_05_coord_manipulation() {
    println!("\n=== TEST 5: Manipulación de coordenadas ===");
    
    let mut pos = Coord::new(0, 0);
    let dest = Coord::new(2, 2);
    
    my_thread_create(
        "CoordTest",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            println!("  [Hilo {}] pos: ({}, {}), dest: ({}, {})", 
                     tid, pos.x, pos.y, dest.x, dest.y);
            
            if pos.x < dest.x {
                pos.x += 1;
                println!("  [Hilo {}] Movió a ({}, {})", tid, pos.x, pos.y);
                ThreadSignal::Yield
            } else if pos.y < dest.y {
                pos.y += 1;
                println!("  [Hilo {}] Movió a ({}, {})", tid, pos.x, pos.y);
                ThreadSignal::Yield
            } else {
                println!("  [Hilo {}] Llegó a destino!", tid);
                ThreadSignal::Exit
            }
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    println!("  ✓ Test 5 pasó\n");
}

#[test]
fn test_06_agent_state_machine() {
    println!("\n=== TEST 6: Máquina de estados de agente ===");
    
    #[derive(Debug, Clone, Copy)]
    enum TestState {
        Init,
        Moving,
        Done,
    }
    
    let mut state = TestState::Init;
    let mut steps = 0u32;
    
    my_thread_create(
        "StateMachine",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            println!("  [Hilo {}] Estado: {:?}, paso: {}", tid, state, steps);
            
            match state {
                TestState::Init => {
                    println!("  [Hilo {}] Init -> Moving", tid);
                    state = TestState::Moving;
                    ThreadSignal::Yield
                }
                TestState::Moving => {
                    steps += 1;
                    if steps < 3 {
                        println!("  [Hilo {}] Moviendo (paso {})", tid, steps);
                        ThreadSignal::Yield
                    } else {
                        println!("  [Hilo {}] Moving -> Done", tid);
                        state = TestState::Done;
                        ThreadSignal::Yield
                    }
                }
                TestState::Done => {
                    println!("  [Hilo {}] Terminado", tid);
                    ThreadSignal::Exit
                }
            }
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    println!("  ✓ Test 6 pasó\n");
}

#[test]
fn test_07_bridge_access() {
    println!("\n=== TEST 7: Acceso a puente (sin cruzar) ===");
    
    let (city, _layout) = create_city();
    let shared_city = create_shared_city(city);
    let city_clone = Arc::clone(&shared_city);
    
    my_thread_create(
        "BridgeReader",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            println!("  [Hilo {}] Accediendo a puente...", tid);
            
            let city_lock = city_clone.lock().unwrap();
            if let Some(bridge) = city_lock.get_bridge(1) {
                println!("  [Hilo {}] Puente 1 encontrado: tipo {:?}", tid, bridge.bridge_type);
            }
            drop(city_lock);
            
            ThreadSignal::Exit
        }),
    );
    
    println!("  Ejecutando 3 ciclos...");
    run_simulation(3);
    println!("  ✓ Test 7 pasó\n");
}

#[test]
fn test_08_simple_movement() {
    println!("\n=== TEST 8: Movimiento simple (sin puente) ===");
    
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);
    let city_clone = Arc::clone(&shared_city);
    let layout_clone = layout.clone();
    
    let mut pos = Coord::new(0, 0);
    let dest = Coord::new(0, 1); // Movimiento simple en misma fila
    
    my_thread_create(
        "SimpleMove",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            println!("  [Hilo {}] pos: ({}, {}), dest: ({}, {})", 
                     tid, pos.x, pos.y, dest.x, dest.y);
            
            if pos.x == dest.x && pos.y == dest.y {
                println!("  [Hilo {}] ✓ Llegó a destino", tid);
                return ThreadSignal::Exit;
            }
            
            // Moverse
            if pos.y < dest.y && pos.y + 1 != layout_clone.river_column {
                pos.y += 1;
                println!("  [Hilo {}] Se movió a ({}, {})", tid, pos.x, pos.y);
            }
            
            ThreadSignal::Yield
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    println!("  ✓ Test 8 pasó\n");
}

#[test]
fn test_09_two_threads_no_interaction() {
    println!("\n=== TEST 9: Dos hilos sin interacción ===");
    
    let counter1 = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));
    
    let c1 = counter1.clone();
    my_thread_create(
        "Thread1",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            let mut count = c1.lock().unwrap();
            *count += 1;
            let val = *count;
            drop(count);
            
            println!("  [Hilo {}] Count: {}", tid, val);
            
            if val < 2 {
                ThreadSignal::Yield
            } else {
                println!("  [Hilo {}] Terminando", tid);
                ThreadSignal::Exit
            }
        }),
    );
    
    let c2 = counter2.clone();
    my_thread_create(
        "Thread2",
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            let mut count = c2.lock().unwrap();
            *count += 1;
            let val = *count;
            drop(count);
            
            println!("  [Hilo {}] Count: {}", tid, val);
            
            if val < 2 {
                ThreadSignal::Yield
            } else {
                println!("  [Hilo {}] Terminando", tid);
                ThreadSignal::Exit
            }
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    
    println!("  Thread1 ejecutó {} veces", *counter1.lock().unwrap());
    println!("  Thread2 ejecutó {} veces", *counter2.lock().unwrap());
    println!("  ✓ Test 9 pasó\n");
}

#[test]
fn test_10_scheduler_types() {
    println!("\n=== TEST 10: Diferentes schedulers ===");
    
    // RoundRobin
    my_thread_create(
        "RR-Thread",
        SchedulerParams::RoundRobin,
        Box::new(|tid| {
            println!("  [RR-{}] Ejecutando", tid);
            ThreadSignal::Exit
        }),
    );
    
    // Lottery
    my_thread_create(
        "Lottery-Thread",
        SchedulerParams::Lottery { tickets: 50 },
        Box::new(|tid| {
            println!("  [Lottery-{}] Ejecutando", tid);
            ThreadSignal::Exit
        }),
    );
    
    // RealTime
    my_thread_create(
        "RT-Thread",
        SchedulerParams::RealTime { deadline: 10000 },
        Box::new(|tid| {
            println!("  [RT-{}] Ejecutando", tid);
            ThreadSignal::Exit
        }),
    );
    
    println!("  Ejecutando 10 ciclos...");
    run_simulation(10);
    println!("  ✓ Test 10 pasó\n");
}

#[test]
fn test_11_actual_car_logic_minimal() {
    println!("\n=== TEST 11: Lógica de carro (mínima) ===");
    
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);
    let city_clone = Arc::clone(&shared_city);
    let layout_clone = layout.clone();
    
    let mut pos = Coord::new(0, 0);
    let dest = Coord::new(0, 1);
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum CarState {
        Moving,
        Arrived,
    }
    
    let mut state = CarState::Moving;
    
    my_thread_create(
        "CarTest",
        SchedulerParams::Lottery { tickets: 10 },
        Box::new(move |tid| {
            println!("  [Carro-{}] Estado: {:?}, pos: ({}, {})", tid, state, pos.x, pos.y);
            
            match state {
                CarState::Moving => {
                    // Verificar si llegó
                    if pos.x == dest.x && pos.y == dest.y {
                        println!("  [Carro-{}] ✅ LLEGÓ a destino", tid);
                        state = CarState::Arrived;
                        return ThreadSignal::Exit;
                    }
                    
                    // Moverse (lógica simplificada)
                    if pos.y < dest.y && pos.y + 1 != layout_clone.river_column {
                        pos.y += 1;
                    } else if pos.y > dest.y && pos.y - 1 != layout_clone.river_column {
                        pos.y -= 1;
                    } else if pos.x < dest.x {
                        pos.x += 1;
                    } else if pos.x > dest.x {
                        pos.x -= 1;
                    }
                    
                    ThreadSignal::Yield
                }
                CarState::Arrived => ThreadSignal::Exit,
            }
        }),
    );
    
    println!("  Ejecutando 20 ciclos...");
    run_simulation(20);
    println!("  ✓ Test 11 pasó\n");
}

#[test]
fn test_12_cargo_truck_minimal() {
    println!("\n=== TEST 12: Camión de carga (mínimo) ===");
    
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);
    let city_clone = Arc::clone(&shared_city);
    
    let mut pos = Coord::new(0, 0);
    let dest = Coord::new(1, 0); // Planta 1
    let cargo = SupplyKind::Water;
    
    my_thread_create(
        "TruckTest",
        SchedulerParams::RealTime { deadline: 15000 },
        Box::new(move |tid| {
            println!("  [Camión-{}] pos: ({}, {}), dest: ({}, {})", 
                     tid, pos.x, pos.y, dest.x, dest.y);
            
            if pos.x == dest.x && pos.y == dest.y {
                println!("  [Camión-{}] En planta, entregando {:?}", tid, cargo);
                
                let mut city_lock = city_clone.lock().unwrap();
                let current_time = city_lock.current_time();
                
                if let Some(plant) = city_lock.find_plant_at(dest) {
                    let supply = plant.requires.iter()
                        .find(|s| s.kind == cargo)
                        .expect("Suministro no requerido")
                        .clone();
                    
                    plant.commit_delivery(supply, current_time);
                    println!("  [Camión-{}] ✅ Entrega completada", tid);
                }
                drop(city_lock);
                
                return ThreadSignal::Exit;
            }
            
            // Moverse
            if pos.x < dest.x {
                pos.x += 1;
            } else if pos.x > dest.x {
                pos.x -= 1;
            } else if pos.y < dest.y {
                pos.y += 1;
            } else if pos.y > dest.y {
                pos.y -= 1;
            }
            
            ThreadSignal::Yield
        }),
    );
    
    println!("  Ejecutando 20 ciclos...");
    run_simulation(20);
    println!("  ✓ Test 12 pasó\n");
}