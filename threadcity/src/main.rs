use std::rc::Rc;
use std::cell::RefCell;

use threadcity::cityconfig::create_threadcity;
use threadcity::agents::{AgentDowncast, AgentState, Car, Ambulance, Boat};

use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::{SchedulerType, ThreadEntry};
use mypthreads::api_rust::*;
use mypthreads::signals::ThreadSignal;

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           ThreadCity con MyPthreads                        ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let (city, layout) = create_threadcity();
    let shared_city = Rc::new(RefCell::new(city));
    let mut runtime = ThreadRuntime::new();

    // Grid 5x5, río en columna 2, puentes en filas 1, 2, 3
    let agents_to_spawn: Vec<Box<dyn AgentDowncast + Send>> = vec![
        Box::new(Car::new(100, (0, layout.bridge1_row), (4, layout.bridge1_row))),
        Box::new(Car::new(101, (0, layout.bridge2_row), (4, layout.bridge2_row))),
        Box::new(Ambulance::new(200, (0, layout.bridge3_row), (4, layout.bridge3_row))),
        Box::new(Boat::new(300, (layout.bridge2_row, 0), (layout.bridge2_row, 4))),
    ];

    for mut agent in agents_to_spawn {
        let city_clone = shared_city.clone();
        let agent_name = format!("Agent-{}", agent.id());
        let thread_name = agent_name.clone();

        let mut state = AgentState::Traveling;
        let mut crossing_progress = 0u32;
        let river_col = layout.river_column;
        let is_boat = agent.as_any().downcast_ref::<Boat>().is_some();

        let agent_logic: ThreadEntry = Box::new(move |rt, _| {
            let pos = agent.pos();
            
            match state {
                AgentState::Traveling => {
                    // Condición de llegada genérica
                    let arrived = if is_boat {
                        pos.y >= 4 // Barco navega en Y, llega a columna 4
                    } else {
                        pos.x >= 4 // Carros/ambulancias en X, llegan a fila 4
                    };
                    
                    if arrived {
                        println!("[{}]  LLEGÓ a destino (pos: {:?})", agent_name, pos);
                        return ThreadSignal::Exit;
                    }
                    
                    // Detectar entrada al puente (justo antes del río en columna 2)
                    let at_bridge = if is_boat {
                        // Barco detecta puente por columna Y
                        pos.y == river_col - 1
                    } else {
                        // Carros/ambulancias por fila X
                        pos.x == river_col - 1
                    };
                    
                    if at_bridge {
                        let vehicle_type = if is_boat { " Barco" } 
                            else if agent.as_any().downcast_ref::<Ambulance>().is_some() { "🚑 Ambulancia" }
                            else { "🚗 Carro" };
                        println!("[{}] {} en entrada del puente (pos: {:?})", agent_name, vehicle_type, pos);
                        state = AgentState::WaitingForBridge;
                        return my_thread_yield();
                    }
                    
                    agent.step(100);
                    my_thread_yield()
                }
                
                AgentState::WaitingForBridge => {
                    println!("[{}]  Intentando cruzar puente...", agent_name);
                    
                    let mut city = city_clone.borrow_mut();
                    
                    // Determinar qué puente usar
                    let bridge_idx = if is_boat {
                        1 // Barco usa puente 2 (índice 1)
                    } else {
                        let y = pos.y;
                        if y == layout.bridge1_row { 0 }
                        else if y == layout.bridge2_row { 1 }
                        else { 2 }
                    };
                    
                    let bridge = &mut city.bridges[bridge_idx];
                    
                    // Barco usa request_pass_boat, otros usan request_pass_vehicle
                    let signal = if is_boat {
                        bridge.request_pass_boat(rt)
                    } else {
                        bridge.request_pass_vehicle(rt)
                    };
                    
                    if signal == ThreadSignal::Continue {
                        state = AgentState::CrossingBridge;
                        crossing_progress = 0;
                        my_thread_yield()
                    } else {
                        signal
                    }
                }
                
                AgentState::CrossingBridge => {
                    crossing_progress += 1;
                    agent.step(100);
                    
                    if crossing_progress >= 2 {
                        println!("[{}]  Terminó de cruzar, liberando puente", agent_name);
                        
                        let mut city = city_clone.borrow_mut();
                        
                        let bridge_idx = if is_boat {
                            1
                        } else {
                            let y = pos.y;
                            if y == layout.bridge1_row { 0 }
                            else if y == layout.bridge2_row { 1 }
                            else { 2 }
                        };
                        
                        let bridge = &mut city.bridges[bridge_idx];
                        
                        if is_boat {
                            bridge.release_pass_boat(rt);
                        } else {
                            bridge.release_pass_vehicle(rt);
                        }
                        
                        state = AgentState::Traveling;
                    }
                    
                    my_thread_yield()
                }
                
                AgentState::Arrived => {
                    ThreadSignal::Exit
                }
            }
        });

        my_thread_create(&mut runtime, &thread_name, SchedulerType::RoundRobin, agent_logic, None, None);
    }

    let mut tick = 0;
    const MAX_TICKS: u32 = 200;

    println!("\n--- Corriendo simulación ---\n");
    while !runtime.ready.is_empty() && tick < MAX_TICKS {
        runtime.run_once();
        runtime.advance_time(10);

        if tick % 20 == 0 {
             println!("  Tick: {}, Hilos activos: {}, Tiempo: {}ms", 
                      tick, runtime.ready.len(), runtime.now());
        }

        tick += 1;
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           Simulación Finalizada                           ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Ticks totales: {:>43} ║", tick);
    println!("║ Hilos restantes: {:>40} ║", runtime.ready.len());
    println!("║ Tiempo simulado: {:>39} ms ║", runtime.now());
    println!("╚════════════════════════════════════════════════════════════╝\n");
}