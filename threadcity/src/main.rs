// threadcity/src/main.rs
// REFACTORIZADO: Usa MyMutex en lugar de std::sync::Mutex

// --- IMPORTACIONES ---
use mypthreads::{
    mypthreads_api::{my_thread_chsched, my_thread_create, SchedulerParams, RUNTIME},
    ThreadId, ThreadSignal,
};
use rand::{prelude::*, rng};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;
use threadcity::{
    create_city, create_shared_city, nearest_bridge, AgentInfo, AgentState, AgentType, Ambulance,
    Boat, Car, CargoTruck, CityLayout, Coord, PlantStatus, SharedCity, SupplyKind,
    TrafficDirection, Vehicle,
};

// --- CONTADOR GLOBAL DE IDs ---
static NEXT_AGENT_ID: AtomicU32 = AtomicU32::new(301);
fn get_next_agent_id() -> u32 {
    NEXT_AGENT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           ThreadCity - Simulación Preemptiva              ║");
    println!("║          [REFACTORIZADO: Usando MyMutex]                  ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // --- SETUP ---
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);

    // --- CONTADORES TOTALES ---
    // NOTA: AtomicU32 no necesita mutex, se mantiene igual
    let total_cars = std::sync::Arc::new(AtomicU32::new(0));
    let total_ambulances = std::sync::Arc::new(AtomicU32::new(0));
    let total_trucks = std::sync::Arc::new(AtomicU32::new(0));
    let total_boats = std::sync::Arc::new(AtomicU32::new(0));

    println!("Iniciando simulación...\n");

    // --- CREACIÓN INICIAL DE AGENTES ---
    for i in 0..5 {
        spawn_car(
            i + 1,
            &layout,
            &shared_city,
            std::sync::Arc::clone(&total_cars),
        );
    }
    for i in 0..2 {
        spawn_ambulance(
            i + 100,
            &layout,
            &shared_city,
            std::sync::Arc::clone(&total_ambulances),
        );
    }
    println!("Creando camiones de carga aleatorios...");
    for i in 0..4 {
        spawn_cargo_truck(
            200 + i,
            &layout,
            &shared_city,
            std::sync::Arc::clone(&total_trucks),
        );
    }
    spawn_boat(
        300,
        &layout,
        &shared_city,
        std::sync::Arc::clone(&total_boats),
    );

    println!("Agentes iniciales creados.");
    println!();

    // --- PARÁMETROS DE SIMULACIÓN ---
    const SIMULATION_STEPS: u32 = 100;
    const TIME_PER_STEP_MS: u64 = 500;
    const SCHEDULER_CYCLES_PER_STEP: usize = 10;
    println!(
        "Iniciando simulación... Pasos: {}, Tiempo/Paso: {}ms\n",
        SIMULATION_STEPS, TIME_PER_STEP_MS
    );

    // --- BUCLE PRINCIPAL DE SIMULACIÓN ---
    for step in 0..SIMULATION_STEPS {
        // CAMBIO: Usar try_enter (no bloqueante) con retry desde el hilo principal
        let new_agents = {
            // Intentar adquirir el lock, si no se puede, esperar un poco y reintentar
            let mut city_lock = loop {
                if let Some(lock) = shared_city.try_enter() {
                    break lock;
                }
                // Si no pudimos adquirir el lock, dar tiempo a los hilos
                thread::sleep(Duration::from_micros(100));
            };

            city_lock.update(TIME_PER_STEP_MS);
            city_lock.check_plant_deadlines();
            println!(
                "\n--- [Paso {} | Tiempo: {}ms] ---",
                step,
                city_lock.current_time()
            );
            let agents = city_lock.update_spawner();
            drop(city_lock);
            let _ = shared_city.request_unlock();
            agents
        };

        for agent_type in new_agents {
            let new_id = get_next_agent_id();
            match agent_type {
                AgentType::Car => spawn_car(
                    new_id,
                    &layout,
                    &shared_city,
                    std::sync::Arc::clone(&total_cars),
                ),
                AgentType::Ambulance => spawn_ambulance(
                    new_id,
                    &layout,
                    &shared_city,
                    std::sync::Arc::clone(&total_ambulances),
                ),
                AgentType::Boat => spawn_boat(
                    new_id,
                    &layout,
                    &shared_city,
                    std::sync::Arc::clone(&total_boats),
                ),
                AgentType::CargoTruck(_) => {}
            }
        }

        {
            let tids_to_promote: Vec<u32> = {
                // CAMBIO: Usar try_enter con retry
                let city_lock = loop {
                    if let Some(lock) = shared_city.try_enter() {
                        break lock;
                    }
                    thread::sleep(Duration::from_micros(100));
                };

                let mut tids = Vec::new();
                for plant in &city_lock.plants {
                    if plant.status == PlantStatus::AtRisk {
                        if let Some(needed_supply) = plant.requires.first() {
                            println!(
                                "🚨 EMERGENCIA: Planta {} necesita {:?} urgentemente!",
                                plant.id, needed_supply.kind
                            );
                            for agent_info in city_lock.agents.values() {
                                if let AgentType::CargoTruck(cargo) = agent_info.agent_type {
                                    if cargo == needed_supply.kind {
                                        tids.push(agent_info.vehicle.tid);
                                    }
                                }
                            }
                        }
                    }
                }
                drop(city_lock);
                let _ = shared_city.request_unlock();
                tids
            };
            for tid in tids_to_promote {
                println!("📢 ¡Activando protocolo de emergencia para el Hilo {}!", tid);
                my_thread_chsched(tid, SchedulerParams::Lottery { tickets: 1000 });
            }
        }

        RUNTIME.lock().unwrap().unblock_all_threads();
        RUNTIME.lock().unwrap().run(SCHEDULER_CYCLES_PER_STEP);

        thread::sleep(Duration::from_millis(50));
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              Simulación Finalizada                        ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!(
        "║ Carros Creados: {:>43} ║",
        total_cars.load(Ordering::Relaxed)
    );
    println!(
        "║ Ambulancias Creadas: {:>39} ║",
        total_ambulances.load(Ordering::Relaxed)
    );
    println!(
        "║ Camiones Creados: {:>42} ║",
        total_trucks.load(Ordering::Relaxed)
    );
    println!(
        "║ Barcos Creados: {:>45} ║",
        total_boats.load(Ordering::Relaxed)
    );
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

// --- FUNCIONES SPAWN ---

fn spawn_car(
    id: u32,
    layout: &CityLayout,
    city: &SharedCity,
    counter: std::sync::Arc<AtomicU32>,
) {
    counter.fetch_add(1, Ordering::Relaxed);
    let mut rng = rng();
    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);
    let city_clone = city.clone();
    let layout_clone = layout.clone();
    let mut pos = origin;
    let mut state = AgentState::Traveling;
    let mut crossing_steps = 0u32;

    println!("🚗 Carro-{} creado: {:?} -> {:?}", id, origin, dest);

    let tid = my_thread_create(
        &format!("Car-{}", id),
        SchedulerParams::Lottery { tickets: 10 },
        Box::new(move |tid_interno, current_tickets| {
            vehicle_logic(
                tid_interno,
                id,
                AgentType::Car,
                current_tickets,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
    let agent_info = AgentInfo {
        vehicle: Vehicle::new(id, tid, origin, dest),
        agent_type: AgentType::Car,
    };

    // CAMBIO: Usar try_enter con retry para insertar el agente
    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            let _ = city.request_unlock();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

fn spawn_ambulance(
    id: u32,
    layout: &CityLayout,
    city: &SharedCity,
    counter: std::sync::Arc<AtomicU32>,
) {
    counter.fetch_add(1, Ordering::Relaxed);
    let mut rng = rng();
    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);
    let city_clone = city.clone();
    let layout_clone = layout.clone();
    let mut pos = origin;
    let mut state = AgentState::Traveling;
    let mut crossing_steps = 0u32;

    println!("🚑 Ambulancia-{} creada: {:?} -> {:?}", id, origin, dest);

    let tid = my_thread_create(
        &format!("Ambulance-{}", id),
        SchedulerParams::Lottery { tickets: 100 },
        Box::new(move |tid_interno, current_tickets| {
            vehicle_logic(
                tid_interno,
                id,
                AgentType::Ambulance,
                current_tickets,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
    let ambulance = Ambulance::new(id, tid, (origin.x, origin.y), (dest.x, dest.y));
    let agent_info = AgentInfo {
        vehicle: ambulance.inner,
        agent_type: AgentType::Ambulance,
    };

    // CAMBIO: Usar try_enter con retry
    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            let _ = city.request_unlock();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

fn spawn_cargo_truck(
    id: u32,
    layout: &CityLayout,
    city: &SharedCity,
    counter: std::sync::Arc<AtomicU32>,
) {
    counter.fetch_add(1, Ordering::Relaxed);
    let mut rng = rng();
    let origin = random_position(&mut rng, layout);
    let cargo = random_supply_kind(&mut rng);
    let destination: Coord;
    let deadline: u64;

    {
        // CAMBIO: Usar try_enter con retry
        let city_lock = loop {
            if let Some(lock) = city.try_enter() {
                break lock;
            }
            thread::sleep(Duration::from_micros(100));
        };

        let plant = city_lock
            .plants
            .choose(&mut rng)
            .expect("No hay plantas")
            .clone();
        destination = plant.loc;
        let supply_spec = plant
            .requires
            .iter()
            .find(|s| s.kind == cargo)
            .expect("Suministro no requerido");
        deadline = city_lock.current_time() + supply_spec.deadline_ms;

        drop(city_lock);
        let _ = city.request_unlock();
    }

    let city_clone = city.clone();
    let layout_clone = layout.clone();
    let mut pos = origin;
    let mut state = AgentState::Traveling;
    let mut crossing_steps = 0u32;
    let cargo_for_thread = cargo;

    println!(
        "🚚 CargoTruck-{} ({:?}): {:?} -> {:?}, deadline: {}ms",
        id, cargo, origin, destination, deadline
    );

    let tid = my_thread_create(
        &format!("Truck-{}", id),
        SchedulerParams::RealTime { deadline },
        Box::new(move |tid_interno, current_tickets| {
            cargo_truck_logic(
                tid_interno,
                id,
                cargo_for_thread,
                current_tickets,
                &mut pos,
                destination,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
    let truck = CargoTruck::new(
        id,
        tid,
        (origin.x, origin.y),
        (destination.x, destination.y),
        cargo,
    );
    let agent_info = AgentInfo {
        vehicle: truck.inner,
        agent_type: AgentType::CargoTruck(cargo),
    };

    // CAMBIO: Usar try_enter con retry
    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            let _ = city.request_unlock();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

fn spawn_boat(
    id: u32,
    layout: &CityLayout,
    city: &SharedCity,
    counter: std::sync::Arc<AtomicU32>,
) {
    counter.fetch_add(1, Ordering::Relaxed);
    let city_clone = city.clone();
    let layout_clone = layout.clone();
    let origin = Coord::new(layout.bridge1_row, layout.river_column);
    let dest = Coord::new(layout.bridge3_row + 1, layout.river_column);
    let mut pos = origin;
    let mut state = AgentState::Traveling;
    let mut crossing_steps = 0u32;

    println!("⛵ Barco-{} creado: {:?} -> {:?}", id, origin, dest);

    let tid = my_thread_create(
        &format!("Boat-{}", id),
        SchedulerParams::RoundRobin,
        Box::new(move |tid_interno, current_tickets| {
            boat_logic(
                tid_interno,
                id,
                current_tickets,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
    let boat = Boat::new(id, tid, (origin.x, origin.y), (dest.x, dest.y));
    let agent_info = AgentInfo {
        vehicle: boat.inner,
        agent_type: AgentType::Boat,
    };

    // CAMBIO: Usar try_enter con retry
    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            let _ = city.request_unlock();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

// --- LÓGICA DE AGENTES Y HELPERS ---

fn random_supply_kind(rng: &mut impl Rng) -> SupplyKind {
    if rng.random_bool(0.5) {
        SupplyKind::Radioactive
    } else {
        SupplyKind::Water
    }
}

fn vehicle_logic(
    tid: ThreadId,
    id: u32,
    agent_type: AgentType,
    current_tickets: u32,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Traveling => {
            if pos.x == dest.x && pos.y == dest.y {
                println!("[{}] ✅ Llegó a destino {:?}", id, dest);
                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }
            let needs_bridge = (pos.y < layout.river_column && dest.y > layout.river_column)
                || (pos.y > layout.river_column && dest.y < layout.river_column);
            let at_bridge_entrance = (pos.y == layout.river_column - 1 && dest.y > layout.river_column)
                || (pos.y == layout.river_column + 1 && dest.y < layout.river_column);

            if needs_bridge && at_bridge_entrance {
                println!("[{}] 🚦 En entrada de puente", id);
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }
            move_towards(pos, dest, layout);
            ThreadSignal::Yield
        }
        AgentState::WaitingForBridge => {
            let scheduler_bonus = current_tickets;

            // CAMBIO: Usar try_enter - si no podemos acceder, bloqueamos el hilo
            let Some(city_lock) = city.try_enter() else {
                return ThreadSignal::Block;
            };

            let bridge_id = nearest_bridge(layout, pos.x);
            let bridge = city_lock
                .get_bridge(bridge_id)
                .expect("Puente no encontrado");

            let base_priority = match agent_type {
                AgentType::Ambulance => 100,
                _ => 0,
            };
            let final_priority = base_priority + scheduler_bonus as u8;
            let direction = if pos.y < layout.river_column {
                TrafficDirection::NorthToSouth
            } else {
                TrafficDirection::SouthToNorth
            };

            if agent_type == AgentType::Ambulance {
                println!("[{}] 🚑 AMBULANCIA pasando directamente", id);
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
                drop(city_lock);
                return ThreadSignal::Yield;
            }
            if bridge.try_cross(tid, final_priority, direction) {
                println!("[{}] Comenzó a cruzar puente {}", id, bridge_id);
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
                drop(city_lock);
                ThreadSignal::Continue
            } else {
                drop(city_lock);
                ThreadSignal::Block
            }
        }
        AgentState::CrossingBridge => {
            *crossing_steps += 1;
            if *crossing_steps >= 3 {
                if pos.y < layout.river_column {
                    pos.y = layout.river_column + 1;
                } else {
                    pos.y = layout.river_column - 1;
                }

                // CAMBIO: Usar try_enter
                if let Some(city_lock) = city.try_enter() {
                    let bridge_id = nearest_bridge(layout, pos.x);
                    let bridge = city_lock
                        .get_bridge(bridge_id)
                        .expect("Puente no encontrado");
                    if agent_type != AgentType::Ambulance {
                        bridge.exit_bridge(tid);
                    }
                    drop(city_lock);
                    let _ = city.request_unlock();
                }

                println!("[{}] Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Traveling;
            }
            ThreadSignal::Yield
        }
        AgentState::Arrived => ThreadSignal::Exit,
    }
}

fn cargo_truck_logic(
    tid: ThreadId,
    id: u32,
    cargo: SupplyKind,
    current_tickets: u32,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Traveling => {
            if pos.x == dest.x && pos.y == dest.y {
                // CAMBIO: Usar try_enter
                if let Some(mut city_lock) = city.try_enter() {
                    let current_time = city_lock.current_time();
                    if let Some(plant) = city_lock.find_plant_at(dest) {
                        let supply = plant
                            .requires
                            .iter()
                            .find(|s| s.kind == cargo)
                            .expect("Suministro no requerido")
                            .clone();
                        plant.commit_delivery(supply, current_time);
                        println!(
                            "[Truck-{}] ✅ Entrega de {:?} a Planta en {:?}",
                            id, cargo, dest
                        );
                    }
                    drop(city_lock);
                    let _ = city.request_unlock();
                }
                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }
            let needs_bridge = (pos.y < layout.river_column && dest.y > layout.river_column)
                || (pos.y > layout.river_column && dest.y < layout.river_column);
            let at_bridge_entrance = (pos.y == layout.river_column - 1 && dest.y > layout.river_column)
                || (pos.y == layout.river_column + 1 && dest.y < layout.river_column);
            if needs_bridge && at_bridge_entrance {
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }
            move_towards(pos, dest, layout);
            ThreadSignal::Yield
        }
        AgentState::WaitingForBridge | AgentState::CrossingBridge => vehicle_logic(
            tid,
            id,
            AgentType::CargoTruck(cargo),
            current_tickets,
            pos,
            dest,
            state,
            crossing_steps,
            city,
            layout,
        ),
        AgentState::Arrived => ThreadSignal::Exit,
    }
}

fn boat_logic(
    tid: ThreadId,
    id: u32,
    _current_tickets: u32,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Traveling => {
            if pos.x == dest.x && pos.y == dest.y {
                println!("[Boat-{}] ✅ Llegó a destino {:?}", id, dest);
                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }
            if pos.x == layout.bridge3_row {
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }
            if pos.x < dest.x {
                pos.x += 1;
            }
            ThreadSignal::Yield
        }
        AgentState::WaitingForBridge => {
            // CAMBIO: Usar try_enter
            let Some(city_lock) = city.try_enter() else {
                return ThreadSignal::Block;
            };

            let bridge = city_lock.get_bridge(3).expect("Puente 3 no encontrado");
            if bridge.boat_request_pass() {
                println!("[Boat-{}] ⛵ Puente levadizo levantado, pasando", id);
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
                drop(city_lock);
                ThreadSignal::Continue
            } else {
                drop(city_lock);
                ThreadSignal::Block
            }
        }
        AgentState::CrossingBridge => {
            *crossing_steps += 1;
            if *crossing_steps >= 5 {
                // CAMBIO: Usar try_enter
                if let Some(city_lock) = city.try_enter() {
                    let bridge = city_lock.get_bridge(3).expect("Puente 3 no encontrado");
                    bridge.boat_exit();
                    drop(city_lock);
                    let _ = city.request_unlock();
                }
                pos.x += 1;
                println!("[Boat-{}] ⛵ Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Traveling;
            }
            ThreadSignal::Yield
        }
        AgentState::Arrived => ThreadSignal::Exit,
    }
}

fn move_towards(pos: &mut Coord, dest: Coord, layout: &CityLayout) {
    if pos.y != layout.river_column {
        if pos.y < dest.y && pos.y + 1 != layout.river_column {
            pos.y += 1;
            return;
        } else if pos.y > dest.y && pos.y - 1 != layout.river_column {
            pos.y -= 1;
            return;
        }
    }
    if pos.x < dest.x {
        pos.x += 1;
    } else if pos.x > dest.x {
        pos.x -= 1;
    }
}

fn random_position(rng: &mut impl Rng, layout: &CityLayout) -> Coord {
    let row = rng.gen_range(0..layout.grid_rows);
    let col = if rng.gen_bool(0.5) {
        rng.gen_range(0..layout.river_column)
    } else {
        rng.gen_range((layout.river_column + 1)..layout.grid_cols)
    };
    Coord::new(row, col)
}

fn random_destination(rng: &mut impl Rng, layout: &CityLayout, origin: Coord) -> Coord {
    loop {
        let dest = random_position(rng, layout);
        if dest.x != origin.x || dest.y != origin.y {
            return dest;
        }
    }
}

