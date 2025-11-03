// ============================================================================
// main.rs - ThreadCity Simulación Completa con Barcos y Camiones Aleatorios
// ============================================================================

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use rand::Rng;
use rand_distr::{Distribution, Poisson};

use threadcity::cityconfig::{create_threadcity, CityLayout};
use mypthreads::api_rust::*;
use mypthreads::runtime::ThreadRuntime;
use mypthreads::signals::ThreadSignal;
use mypthreads::thread::{SchedulerType, ThreadEntry, ThreadId};
use threadcity::agents::{Agent, AgentDowncast, AgentState, Ambulance, Car, CargoTruck, Boat};
use threadcity::model::{PlantStatus, SupplyKind, TrafficDirection, Coord};

// ============================================================================
// ESTRUCTURA PARA GENERACIÓN ALEATORIA DE VEHÍCULOS
// ============================================================================

struct VehicleSpawner {
    rng: rand::rngs::ThreadRng,
    poisson: Poisson<f64>,
    vehicles_spawned: u32,
    boats_spawned: u32,
    trucks_spawned: u32,
    next_vehicle_id: u32,
    next_boat_id: u32,
    next_truck_id: u32,
    last_boat_spawn_tick: u32,
    last_truck_spawn_tick: u32,
}

impl VehicleSpawner {
    fn new(mean_spawn_rate: f64) -> Self {
        Self {
            rng: rand::rng(),
            poisson: Poisson::new(mean_spawn_rate).unwrap(),
            vehicles_spawned: 0,
            boats_spawned: 0,
            trucks_spawned: 0,
            next_vehicle_id: 1000,
            next_boat_id: 6000,
            next_truck_id: 5000,
            last_boat_spawn_tick: 0,
            last_truck_spawn_tick: 0,
        }
    }

    fn random_position(&mut self, grid_rows: u32, grid_cols: u32, river_col: u32) -> (u32, u32) {
        let row = self.rng.random_range(0..grid_rows);
        let col = if self.rng.random_bool(0.5) {
            self.rng.random_range(0..river_col)
        } else {
            self.rng.random_range((river_col + 1)..grid_cols)
        };
        (row, col)
    }

    fn random_destination(&mut self, origin: (u32, u32), grid_rows: u32, grid_cols: u32, river_col: u32) -> (u32, u32) {
        let mut dest;
        loop {
            dest = self.random_position(grid_rows, grid_cols, river_col);
            if dest != origin {
                break;
            }
        }
        dest
    }

    fn should_spawn(&mut self) -> bool {
        let sample = self.poisson.sample(&mut self.rng);
        sample > 0.5
    }

    // Generar un barco (cada 100 ticks aproximadamente)
    fn should_spawn_boat(&mut self, current_tick: u32) -> bool {
        current_tick - self.last_boat_spawn_tick >= 100
    }

    fn spawn_boat(&mut self, layout: &CityLayout, current_tick: u32) -> Box<dyn AgentDowncast + Send> {
        let id = self.next_boat_id;
        self.next_boat_id += 1;
        self.boats_spawned += 1;
        self.last_boat_spawn_tick = current_tick;

        // Los barcos aparecen en la columna del río, entre puente 1 y 2
        let start_row = self.rng.random_range(layout.bridge1_row..(layout.bridge2_row + 1));
        let origin = (start_row, layout.river_column);
        
        // El destino es más abajo en el río (atraviesa el puente 3)
        let dest_row = layout.bridge3_row + 1;
        let destination = (dest_row, layout.river_column);

        println!("⛵ Generando Barco-{} en {:?} -> {:?} (pasará por Puente 3)", id, origin, destination);
        Box::new(Boat::new(id, origin, destination))
    }

    // Generar un camión de suministros (cada 80 ticks aproximadamente)
    fn should_spawn_truck(&mut self, current_tick: u32) -> bool {
        current_tick - self.last_truck_spawn_tick >= 20
    }

    fn spawn_cargo_truck(&mut self, layout: &CityLayout, current_tick: u32, plants: &[threadcity::model::NuclearPlant]) -> Box<dyn AgentDowncast + Send> {
        let id = self.next_truck_id;
        self.next_truck_id += 1;
        self.trucks_spawned += 1;
        self.last_truck_spawn_tick = current_tick;

        // Elegir planta aleatoria
        let plant_idx = self.rng.random_range(0..plants.len());
        let plant = &plants[plant_idx];
        let destination = (plant.loc.x, plant.loc.y);

        // Elegir tipo de suministro aleatorio de lo que requiere la planta
        let supply_idx = self.rng.random_range(0..plant.requires.len());
        let cargo = plant.requires[supply_idx].kind;

        // Origen aleatorio (del lado opuesto al destino para cruzar el puente)
        let origin = if destination.1 < layout.river_column {
            // Planta está al Oeste, camión viene del Este
            let row = self.rng.random_range(0..layout.grid_rows);
            let col = self.rng.random_range((layout.river_column + 1)..layout.grid_cols);
            (row, col)
        } else {
            // Planta está al Este, camión viene del Oeste
            let row = self.rng.random_range(0..layout.grid_rows);
            let col = self.rng.random_range(0..layout.river_column);
            (row, col)
        };

        println!("🚚 Generando CargoTruck-{} ({:?}) en {:?} -> Planta {} {:?}", 
                 id, cargo, origin, plant.id, destination);
        Box::new(CargoTruck::new(id, origin, destination, cargo))
    }

    fn spawn_vehicle(&mut self, layout: &CityLayout) -> Box<dyn AgentDowncast + Send> {
        let grid_rows = layout.grid_rows;
        let grid_cols = layout.grid_cols;
        let river_col = layout.river_column;

        let origin = self.random_position(grid_rows, grid_cols, river_col);
        let destination = self.random_destination(origin, grid_rows, grid_cols, river_col);

        let vehicle_type = self.rng.random_range(0..10);
        let id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        self.vehicles_spawned += 1;

        if vehicle_type == 0 {
            println!("🚑 Generando Ambulancia-{} en {:?} -> {:?}", id, origin, destination);
            Box::new(Ambulance::new(id, origin, destination))
        } else {
            println!("🚗 Generando Carro-{} en {:?} -> {:?}", id, origin, destination);
            Box::new(Car::new(id, origin, destination))
        }
    }
}

// ============================================================================
// FUNCIÓN PRINCIPAL
// ============================================================================

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           ThreadCity con MyPthreads                        ║");
    println!("║      Vehículos, Barcos y Camiones Aleatorios              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let (city, layout) = create_threadcity();
    let shared_city = Rc::new(RefCell::new(city));
    let mut runtime = ThreadRuntime::new();
    
    let mut spawner = VehicleSpawner::new(0.3);
    
    let mut supply_truck_threads: HashMap<ThreadId, SupplyKind> = HashMap::new();

    let mut tick = 0;
    const MAX_TICKS: u32 = 1000;
    const MIN_VEHICLES: u32 = 25;

    println!("\n--- Corriendo simulación ---\n");
    
    while (!runtime.ready.is_empty() || spawner.vehicles_spawned < MIN_VEHICLES) && tick < MAX_TICKS {
        // Generar vehículos normales
        if spawner.vehicles_spawned < MIN_VEHICLES && spawner.should_spawn() {
            let agent = spawner.spawn_vehicle(&layout);
            spawn_agent(agent, &shared_city, &layout, &mut runtime, &mut supply_truck_threads);
        }

        // Generar barcos cada ~100 ticks
        if spawner.should_spawn_boat(tick) {
            let boat = spawner.spawn_boat(&layout, tick);
            spawn_agent(boat, &shared_city, &layout, &mut runtime, &mut supply_truck_threads);
        }

        // Generar camiones de suministro cada ~80 ticks
        if spawner.should_spawn_truck(tick) {
            let truck = {
                let city = shared_city.borrow();
                spawner.spawn_cargo_truck(&layout, tick, &city.plants)
            };
            spawn_agent(truck, &shared_city, &layout, &mut runtime, &mut supply_truck_threads);
        }

        runtime.run_once();
        const TIME_STEP_MS: u64 = 100;
        runtime.advance_time(TIME_STEP_MS);

        {
            let mut city = shared_city.borrow_mut();
            
            for bridge in city.bridges.iter_mut() {
                bridge.step(TIME_STEP_MS, &mut runtime);
            }
            
            for plant in city.plants.iter_mut() {
                if plant.status == PlantStatus::Exploded {
                    continue;
                }

                for supply in &plant.requires {
                    let last_delivery_time = plant.get_last_delivery_time(&supply.kind);
                    let deadline = last_delivery_time + supply.deadline_ms;
                    let fail_time = deadline + plant.deadline_policy.max_lateness_ms;
                    let time_until_failure = fail_time.saturating_sub(runtime.now());

                    let emergency_threshold = plant.deadline_policy.max_lateness_ms * 3 / 10;
                    
                    if time_until_failure < emergency_threshold && plant.status != PlantStatus::AtRisk {
                        plant.status = PlantStatus::AtRisk;
                        println!("\n⚠️⚠️⚠️ ALERTA: Planta {} en riesgo! Falta {:?}", 
                                 plant.id, supply.kind);
                        println!("    Tiempo restante: {}ms", time_until_failure);
                        
                        for (tid, cargo_kind) in supply_truck_threads.iter() {
                            if *cargo_kind == supply.kind {
                                println!("🚨 Cambiando scheduler del camión {} de RealTime a Lottery con 1000 tickets", tid);
                                my_thread_chsched(&mut runtime, *tid, SchedulerType::Lottery, Some(1000), None);
                            }
                        }
                    }

                    if runtime.now() > fail_time {
                        plant.status = PlantStatus::Exploded;
                        println!("\n☢️☢️☢️ ¡BOOM! La planta nuclear {} ha explotado por falta de {:?}!", 
                                 plant.id, supply.kind);
                        println!("    Tiempo Límite Excedido: {}ms > {}ms (Última entrega en {}ms)\n",
                                 runtime.now(), fail_time, last_delivery_time);
                        break;
                    }
                }
            }
        }

        if tick % 50 == 0 {
            println!("  Tick: {}, Hilos: {}, Vehículos: {}, Camiones: {}, Barcos: {}, Tiempo: {}ms",
                     tick, runtime.ready.len(), spawner.vehicles_spawned, 
                     spawner.trucks_spawned, spawner.boats_spawned, runtime.now());
        }

        tick += 1;
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           Simulación Finalizada                           ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Ticks totales: {:>43} ║", tick);
    println!("║ Vehículos generados: {:>37} ║", spawner.vehicles_spawned);
    println!("║ Camiones generados: {:>38} ║", spawner.trucks_spawned);
    println!("║ Barcos generados: {:>40} ║", spawner.boats_spawned);
    println!("║ Hilos restantes: {:>40} ║", runtime.ready.len());
    println!("║ Tiempo simulado: {:>39} ms ║", runtime.now());
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

// ============================================================================
// FUNCIÓN PARA CREAR Y SPAWNER AGENTES
// ============================================================================

fn spawn_agent(
    mut agent: Box<dyn AgentDowncast + Send>,
    shared_city: &Rc<RefCell<threadcity::sim::City>>,
    layout: &CityLayout,
    runtime: &mut ThreadRuntime,
    supply_truck_threads: &mut HashMap<ThreadId, SupplyKind>,
) -> ThreadId {
    let city_clone = shared_city.clone();
    let agent_name = format!("Agent-{}", agent.id());
    let thread_name = agent_name.clone();

    let (scheduler, tickets, deadline, cargo_kind) =
        if let Some(truck) = agent.as_any().downcast_ref::<CargoTruck>() {
            println!("🚚 Creando hilo RealTime para CargoTruck-{} (cargo: {:?})...", truck.id(), truck.cargo);
            let city = shared_city.borrow();
            
            // Encontrar la planta destino
            let dest_coord = Coord::new(truck.inner.destination.x, truck.inner.destination.y);
            let plant = city.plants.iter()
                .find(|p| p.loc.x == dest_coord.x && p.loc.y == dest_coord.y)
                .expect("No se encontró la planta destino");

            let supply_spec = plant
                .requires
                .iter()
                .find(|s| s.kind == truck.cargo)
                .expect("La planta no requiere el suministro");

            let absolute_deadline = runtime.now() + supply_spec.deadline_ms;
            (SchedulerType::RealTime, None, Some(absolute_deadline), Some(truck.cargo))
        } else if agent.as_any().downcast_ref::<Ambulance>().is_some() {
            println!("🚑 Creando hilo Lottery (200 tickets) para Ambulancia-{}...", agent.id());
            (SchedulerType::Lottery, Some(200), None, None)
        } else if agent.as_any().downcast_ref::<Boat>().is_some() {
            println!("⛵ Creando hilo RoundRobin para Barco-{}...", agent.id());
            (SchedulerType::RoundRobin, Some(10), None, None)
        } else {
            (SchedulerType::Lottery, Some(10), None, None)
        };

    let mut state = AgentState::Traveling;
    let mut crossing_progress = 0u32;
    let river_col = layout.river_column;
    let is_truck = agent.as_any().downcast_ref::<CargoTruck>().is_some();
    let is_ambulance = agent.as_any().downcast_ref::<Ambulance>().is_some();
    let is_boat = agent.as_any().downcast_ref::<Boat>().is_some();
    let bridge1_row = layout.bridge1_row;
    let bridge2_row = layout.bridge2_row;
    let bridge3_row = layout.bridge3_row;

    let agent_logic: ThreadEntry = Box::new(move |rt, _| {
        let pos = agent.pos();
        
        let dest = if let Some(car) = agent.as_any().downcast_ref::<Car>() {
            (car.inner.destination.x, car.inner.destination.y)
        } else if let Some(amb) = agent.as_any().downcast_ref::<Ambulance>() {
            (amb.inner.destination.x, amb.inner.destination.y)
        } else if let Some(truck) = agent.as_any().downcast_ref::<CargoTruck>() {
            (truck.inner.destination.x, truck.inner.destination.y)
        } else if let Some(boat) = agent.as_any().downcast_ref::<Boat>() {
            (boat.inner.destination.x, boat.inner.destination.y)
        } else {
            (4, 4)
        };

        match state {
            AgentState::Traveling => {
                if pos.x == dest.0 && pos.y == dest.1 {
                    println!("[{}] ✅ LLEGÓ a destino (pos: {:?})", agent_name, pos);

                    if let Some(truck) = agent.as_any().downcast_ref::<CargoTruck>() {
                        let mut city = city_clone.borrow_mut();
                        // Encontrar la planta en la posición de destino
                        if let Some(plant) = city.plants.iter_mut()
                            .find(|p| p.loc.x == dest.0 && p.loc.y == dest.1) {
                            let supply_spec = plant
                                .requires
                                .iter()
                                .find(|s| s.kind == truck.cargo)
                                .unwrap()
                                .clone();
                            plant.commit_delivery(supply_spec, rt.now());
                            println!("✅ [{}] Entrega de {:?} registrada en Planta {} en tiempo {}ms", 
                                     agent_name, truck.cargo, plant.id, rt.now());
                        }
                    }
                    return ThreadSignal::Exit;
                }

                // LÓGICA ESPECIAL PARA BARCOS
                if is_boat {
                    // Los barcos solo se mueven verticalmente en el río
                    let at_bridge3 = pos.x == bridge3_row && pos.y == river_col;
                    
                    if at_bridge3 {
                        println!("[{}] ⛵ Barco llegando al Puente Levadizo 3", agent_name);
                        state = AgentState::WaitingForBridge;
                        return my_thread_yield();
                    }

                    // Moverse verticalmente en el río
                    let mut new_pos = pos;
                    if pos.x < dest.0 {
                        new_pos.x += 1;
                    } else if pos.x > dest.0 {
                        new_pos.x -= 1;
                    }
                    agent.set_pos(new_pos);
                    return my_thread_yield();
                }

                // LÓGICA PARA VEHÍCULOS TERRESTRES
                let needs_to_cross = (pos.y < river_col && dest.1 > river_col)
                    || (pos.y > river_col && dest.1 < river_col);

                let at_bridge_entrance = if dest.1 < river_col {
                    pos.y == river_col + 1
                } else if dest.1 > river_col {
                    pos.y == river_col - 1
                } else {
                    false
                };

                if needs_to_cross && at_bridge_entrance {
                    println!("[{}] 🚦 En entrada del puente (pos: {:?})", agent_name, pos);
                    state = AgentState::WaitingForBridge;
                    return my_thread_yield();
                }

                let mut new_pos = pos;
                if pos.y < dest.1 {
                    new_pos.y += 1;
                } else if pos.y > dest.1 {
                    new_pos.y -= 1;
                } else if pos.x < dest.0 {
                    new_pos.x += 1;
                } else if pos.x > dest.0 {
                    new_pos.x -= 1;
                }

                agent.set_pos(new_pos);
                my_thread_yield()
            }

            AgentState::WaitingForBridge => {
                // LÓGICA PARA BARCOS
                if is_boat {
                    println!("[{}] ⛵ Barco solicitando paso por Puente Levadizo 3", agent_name);
                    let mut city = city_clone.borrow_mut();
                    let bridge = &mut city.bridges[2]; // Puente 3 (índice 2)
                    let signal = bridge.request_pass_boat(rt);

                    if signal == ThreadSignal::Continue {
                        println!("[{}] ⛵ Puente levadizo levantado, barco pasando", agent_name);
                        state = AgentState::CrossingBridge;
                        crossing_progress = 0;
                    }
                    return signal;
                }

                // AMBULANCIAS SIEMPRE SE SALTAN LOS SEMÁFOROS
                if is_ambulance {
                    println!("[{}] 🚑 AMBULANCIA: Pasando SIN esperar semáforo!", agent_name);
                    state = AgentState::CrossingBridge;
                    crossing_progress = 0;
                    return my_thread_yield();
                }

                // CAMIONES SOLO SE SALTAN EN EMERGENCIA
                if is_truck {
                    let is_emergency = {
                        let city = city_clone.borrow();
                        city.plants.iter().any(|p| p.status == PlantStatus::AtRisk)
                    };

                    if is_emergency {
                        println!("[{}] 🚨 CAMIÓN EN EMERGENCIA: Ignorando semáforo!", agent_name);
                        state = AgentState::CrossingBridge;
                        crossing_progress = 0;
                        return my_thread_yield();
                    }
                }

                // CARROS NORMALES ESPERAN
                println!("[{}] Intentando cruzar puente...", agent_name);
                let mut city = city_clone.borrow_mut();

                let nearest_bridge_row =
                    [bridge1_row, bridge2_row, bridge3_row]
                        .iter()
                        .min_by_key(|&&row| (pos.x as i32 - row as i32).abs())
                        .copied()
                        .unwrap_or(pos.x);

                let bridge_idx = if nearest_bridge_row == bridge1_row {
                    0
                } else if nearest_bridge_row == bridge2_row {
                    1
                } else {
                    2
                };

                let bridge = &mut city.bridges[bridge_idx];
                let direction = if pos.y < river_col {
                    TrafficDirection::WestToEast
                } else {
                    TrafficDirection::EastToWest
                };

                let agent_priority = agent.priority();
                let signal = bridge.request_pass_vehicle(rt, agent_priority, direction);

                if signal == ThreadSignal::Continue {
                    state = AgentState::CrossingBridge;
                    crossing_progress = 0;
                }

                signal
            }

            AgentState::CrossingBridge => {
                crossing_progress += 1;

                let crossing_time = if is_boat { 5 } else { 3 };

                if crossing_progress >= crossing_time {
                    println!("[{}] Terminó de cruzar", agent_name);
                    let mut new_pos = agent.pos();

                    if is_boat {
                        // Barco continúa en el río, avanzar verticalmente
                        new_pos.x += 1;
                        agent.set_pos(new_pos);
                        
                        let mut city = city_clone.borrow_mut();
                        let bridge = &mut city.bridges[2]; // Puente 3
                        bridge.release_pass_boat(rt);
                    } else {
                        // Vehículo terrestre cruza horizontalmente
                        if new_pos.y < river_col {
                            new_pos.y = river_col + 1;
                        } else {
                            new_pos.y = river_col - 1;
                        }
                        agent.set_pos(new_pos);

                        if !is_ambulance {
                            let mut city = city_clone.borrow_mut();
                            let nearest_bridge_row =
                                [bridge1_row, bridge2_row, bridge3_row]
                                    .iter()
                                    .min_by_key(|&&row| (new_pos.x as i32 - row as i32).abs())
                                    .copied()
                                    .unwrap_or(new_pos.x);

                            let bridge_idx = if nearest_bridge_row == bridge1_row {
                                0
                            } else if nearest_bridge_row == bridge2_row {
                                1
                            } else {
                                2
                            };

                            let bridge = &mut city.bridges[bridge_idx];
                            bridge.release_pass_vehicle(rt);
                        }
                    }

                    println!("[{}] 📍 Nueva posición: {:?}", agent_name, new_pos);
                    state = AgentState::Traveling;
                }
                my_thread_yield()
            }

            AgentState::Arrived => ThreadSignal::Exit,
        }
    });

    let tid = my_thread_create(runtime, &thread_name, scheduler, agent_logic, tickets, deadline);
    
    if let Some(cargo) = cargo_kind {
        supply_truck_threads.insert(tid, cargo);
    }
    
    tid
}