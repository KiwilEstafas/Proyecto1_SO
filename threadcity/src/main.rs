use std::rc::Rc;
use std::cell::RefCell;

use threadcity::cityconfig::create_threadcity;
// Importamos el trait 'Agent' para poder usar sus métodos como .id()
use threadcity::agents::{Agent, AgentDowncast, AgentState, Car, Ambulance, Boat, CargoTruck};
use threadcity::model::{PlantStatus, SupplyKind};

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

    const PLANT1_COORDS: (u32, u32) = (1, 0);

    let agents_to_spawn: Vec<Box<dyn AgentDowncast + Send>> = vec![
        // Todos van por el mismo puente (fila 1)
        Box::new(Car::new(100, (0, layout.bridge1_row), (4, layout.bridge1_row))),
        Box::new(Car::new(101, (0, layout.bridge1_row), (4, layout.bridge1_row))),
        Box::new(Ambulance::new(200, (0, layout.bridge1_row), (4, layout.bridge1_row))), //  misma fila
        Box::new(CargoTruck::new(501, (0, layout.bridge1_row), (4, layout.bridge1_row), SupplyKind::Water)),
    ];


    for mut agent in agents_to_spawn {
        let city_clone = shared_city.clone();
        let agent_name = format!("Agent-{}", agent.id());
        let thread_name = agent_name.clone();

        let (scheduler, tickets, deadline) = if let Some(truck) = agent.as_any().downcast_ref::<CargoTruck>() {
            println!("🚚 Creando hilo de Tiempo Real para CargoTruck-{}...", truck.id());
            let city = shared_city.borrow();
            let plant = &city.plants[0];
            
            let supply_spec = plant.requires.iter()
                .find(|s| s.kind == truck.cargo)
                .expect("La planta no requiere el suministro que transporta el camión");
            
            let absolute_deadline = runtime.now() + supply_spec.deadline_ms;
            (SchedulerType::RealTime, None, Some(absolute_deadline))

        } else if agent.as_any().downcast_ref::<Ambulance>().is_some() {
            println!("🚑 Creando hilo de Lotería con alta prioridad para Ambulancia-{}...", agent.id());
            (SchedulerType::Lottery, Some(100), None)

        } else {
            (SchedulerType::RoundRobin, Some(10), None)
        };
        
        let mut state = AgentState::Traveling;
        let mut crossing_progress = 0u32;
        let river_col = layout.river_column;
        let is_boat = agent.as_any().downcast_ref::<Boat>().is_some();
        let is_truck = agent.as_any().downcast_ref::<CargoTruck>().is_some();

        let agent_logic: ThreadEntry = Box::new(move |rt, _| {
            let pos = agent.pos();
            let dest = if is_truck { PLANT1_COORDS } else if is_boat { (pos.x, 4) } else { (4, pos.y) };

            match state {
                AgentState::Traveling => {
                    if pos.x == dest.0 && pos.y == dest.1 {
                        println!("[{}]  LLEGÓ a destino (pos: {:?})", agent_name, pos);

                        if let Some(truck) = agent.as_any().downcast_ref::<CargoTruck>() {
                            let mut city = city_clone.borrow_mut();
                            if let Some(plant) = city.plants.get_mut(0) {
                                let supply_spec = plant.requires.iter().find(|s| s.kind == truck.cargo).unwrap().clone();
                                plant.commit_delivery(supply_spec, rt.now());
                                println!("✅ [{}] Entrega de {:?} registrada en la planta.", agent_name, truck.cargo);
                            }
                        }
                        return ThreadSignal::Exit;
                    }
                    
                    let needs_to_cross = (pos.x < river_col && dest.0 >= river_col) || (pos.x >= river_col && dest.0 < river_col);
                    let at_bridge_entrance = pos.x == river_col - 1;

                    if needs_to_cross && at_bridge_entrance {
                        println!("[{}] en entrada del puente (pos: {:?})", agent_name, pos);
                        state = AgentState::WaitingForBridge;
                        return my_thread_yield();
                    }
                    
                    agent.step(100);
                    my_thread_yield()
                }
                
                AgentState::WaitingForBridge => {
                    println!("[{}]  Intentando cruzar puente...", agent_name);
                    let mut city = city_clone.borrow_mut();
                    
                    // Encontrar el puente más cercano (igual que antes)
                    let nearest_bridge_row = [layout.bridge1_row, layout.bridge2_row, layout.bridge3_row]
                        .iter()
                        .min_by_key(|&&row| (pos.y as i32 - row as i32).abs())
                        .map(|&val| val)
                        .unwrap_or(pos.y);

                    let bridge_idx = if nearest_bridge_row == layout.bridge1_row {
                        0
                    } else if nearest_bridge_row == layout.bridge2_row {
                        1
                    } else {
                        2
                    };

                    let bridge = &mut city.bridges[bridge_idx];

                    // Pasamos la prioridad real del agente al puente.
                    // Si tu trait `Agent` no tiene todavía `fn priority(&self) -> u8`,
                    // agregalo al trait (y devolvé self.priority en Vehicle, etc.)
                    let agent_priority = agent.priority();

                    let signal = bridge.request_pass_vehicle(rt, agent_priority);

                    if signal == ThreadSignal::Continue {
                        state = AgentState::CrossingBridge;
                        crossing_progress = 0;
                    }

                    signal
                }

                AgentState::CrossingBridge => {
                    crossing_progress += 1;
                    
                    if crossing_progress >= 3 {
                        println!("[{}]  Terminó de cruzar, liberando puente", agent_name);
                        let mut pos = agent.pos();   // obtiene una copia
                        pos.x = river_col + 1;       // modifica la copia
                        agent.set_pos(pos);          // la vuelve a escribir en el agente real


                        let mut city = city_clone.borrow_mut();
                        let bridge_idx = 0;
                        let bridge = &mut city.bridges[bridge_idx];
                        bridge.release_pass_vehicle(rt);
                        
                        state = AgentState::Traveling;
                    }
                    my_thread_yield()
                }
                
                AgentState::Arrived => ThreadSignal::Exit,
            }
        });


        my_thread_create(&mut runtime, &thread_name, scheduler, agent_logic, tickets, deadline);
    }

    let mut tick = 0;
    const MAX_TICKS: u32 = 500;

    println!("\n--- Corriendo simulación ---\n");
    while !runtime.ready.is_empty() && tick < MAX_TICKS {
        runtime.run_once();
        runtime.advance_time(10);

        { 
            let mut city = shared_city.borrow_mut();
            for plant in city.plants.iter_mut() {
                if plant.status == PlantStatus::Exploded {
                    continue;
                }

                for supply in &plant.requires {
                    let last_delivery_time = plant.get_last_delivery_time(&supply.kind);
                    let fail_time = last_delivery_time + supply.deadline_ms + plant.deadline_policy.max_lateness_ms;

                    if runtime.now() > fail_time {
                        plant.status = PlantStatus::Exploded;
                        println!("\n ¡BOOM! La planta nuclear {} ha explotado por falta de {:?}!", plant.id, supply.kind);
                        println!("    Tiempo Límite Excedido: {}ms > {}ms (Última entrega en {}ms)\n", runtime.now(), fail_time, last_delivery_time);
                        break; 
                    }
                }
            }
        }

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