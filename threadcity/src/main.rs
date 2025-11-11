use mypthreads::{
    mypthreads_api::{
        my_thread_chsched, my_thread_create, runtime_init, runtime_run_cycles, runtime_unblock_all,
        SchedulerParams,
    },
    ThreadId, ThreadSignal,
};
use rand::{prelude::*, rng};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;
use threadcity::{
    create_city, create_shared_city, nearest_bridge, AgentInfo, AgentState, AgentType, Ambulance,
    Boat, CargoTruck, CityLayout, Coord, PlantStatus, SharedCity, SupplyKind,
    TrafficDirection, Vehicle,
};


static NEXT_AGENT_ID: AtomicU32 = AtomicU32::new(301);
fn get_next_agent_id() -> u32 {
    NEXT_AGENT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           ThreadCity - Simulación                           ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // --- SETUP ---
    runtime_init();
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);

    // --- CONTADORES TOTALES ---
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
    const SCHEDULER_CYCLES_PER_STEP: usize = 20; // Aumentado para dar más tiempo a los hilos
    println!(
        "Iniciando simulación... Pasos: {}, Tiempo/Paso: {}ms\n",
        SIMULATION_STEPS, TIME_PER_STEP_MS
    );

    // --- BUCLE PRINCIPAL DE SIMULACIÓN ---
    for step in 0..SIMULATION_STEPS {
        let new_agents = {
            let mut city_lock = loop {
                if let Some(lock) = shared_city.try_enter() {
                    break lock;
                }
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
            shared_city.force_unlock_for_main();
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
                let city_lock = loop {
                    if let Some(lock) = shared_city.try_enter() {
                        break lock;
                    }
                    thread::sleep(Duration::from_micros(100));
                };

                let mut tids = Vec::new();
                for plant in &city_lock.plants {
                    if plant.status == PlantStatus::AtRisk {
                        if let Some(needed_supply) =
                            plant.active_risk_kind(city_lock.current_time())
                        {
                            for agent_info in city_lock.agents.values() {
                                if let AgentType::CargoTruck(cargo) = agent_info.agent_type {
                                    if cargo == needed_supply {
                                        tids.push(agent_info.vehicle.tid);
                                    }
                                }
                            }
                        }
                    }
                }
                drop(city_lock);
                shared_city.force_unlock_for_main();
                tids
            };
            for tid in tids_to_promote {
                println!(
                    "📢 ¡Activando protocolo de emergencia para el Hilo {}!",
                    tid
                );
                my_thread_chsched(tid, SchedulerParams::Lottery { tickets: 1000 });
            }
        }

        runtime_unblock_all();
        runtime_run_cycles(SCHEDULER_CYCLES_PER_STEP);

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
fn spawn_car(id: u32, layout: &CityLayout, city: &SharedCity, counter: std::sync::Arc<AtomicU32>) {
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

    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            city.force_unlock_for_main();
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

    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            city.force_unlock_for_main();
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
        city.force_unlock_for_main();
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

    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            city.force_unlock_for_main();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

fn spawn_boat(id: u32, layout: &CityLayout, city: &SharedCity, counter: std::sync::Arc<AtomicU32>) {
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

    loop {
        if let Some(mut city_lock) = city.try_enter() {
            city_lock.agents.insert(tid, agent_info);
            drop(city_lock);
            city.force_unlock_for_main();
            break;
        }
        thread::sleep(Duration::from_micros(50));
    }
}

// --- LÓGICA DE AGENTES Y HELPERS ---
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
            let at_bridge_entrance = (pos.y == layout.river_column - 1
                && dest.y > layout.river_column)
                || (pos.y == layout.river_column + 1 && dest.y < layout.river_column);

            if needs_bridge && at_bridge_entrance {
                println!("[{}] 🚦 En entrada de puente", id);
                *state = AgentState::WaitingForBridge;
            } else {
                move_towards(pos, dest, layout);
            }
            ThreadSignal::Yield
        }
        AgentState::WaitingForBridge => {
            let city_lock = match city.try_enter() {
                Some(lock) => lock,
                None => return ThreadSignal::Block, // No pude obtener el lock, me bloqueo.
            };

            let bridge_id = nearest_bridge(layout, pos.x);
            let bridge = city_lock
                .get_bridge(bridge_id)
                .expect("Puente no encontrado");
            let direction = if pos.y < layout.river_column {
                TrafficDirection::NorthToSouth
            } else {
                TrafficDirection::SouthToNorth
            };
            let mut can_cross = false;

            if agent_type == AgentType::Ambulance {
                println!("[{}] 🚑 AMBULANCIA pasando directamente", id);
                can_cross = true;
            } else {
                let base_priority = 0;
                let final_priority = base_priority + current_tickets as u8;
                if bridge.try_cross(tid, final_priority, direction) {
                    println!("[{}] Comenzó a cruzar puente {}", id, bridge_id);
                    can_cross = true;
                }
            }

            if can_cross {
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
            }

            // La acción final de este turno es SIEMPRE liberar el lock.
            // El runtime nos pondrá de nuevo en la cola de listos.
            // Si no pudimos cruzar, seguiremos en `WaitingForBridge` y reintentaremos.

            drop(city_lock);
            city.force_unlock_for_main();
            return ThreadSignal::Yield;
        }
        AgentState::CrossingBridge => {
            *crossing_steps += 1;
            if *crossing_steps >= 3 {
                if pos.y < layout.river_column {
                    pos.y = layout.river_column + 1;
                } else {
                    pos.y = layout.river_column - 1;
                }

                println!("[{}] Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Traveling;

                // Notificar al puente es una acción crítica.
                let city_lock = match city.try_enter() {
                    Some(lock) => lock,
                    // Si no puedo notificar ahora, lo intentaré en el siguiente ciclo.
                    // No es ideal, pero no causa deadlock.
                    None => return ThreadSignal::Yield,
                };

                let bridge_id = nearest_bridge(layout, pos.x);
                if let Some(bridge) = city_lock.get_bridge(bridge_id) {
                    if agent_type != AgentType::Ambulance {
                        bridge.exit_bridge(tid);
                    }
                }

                drop(city_lock);
                city.force_unlock_for_main();
                return ThreadSignal::Yield;
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
    if *state != AgentState::Arrived && pos.x == dest.x && pos.y == dest.y {
        let mut city_lock = match city.try_enter() {
            Some(lock) => lock,
            None => return ThreadSignal::Block,
        };

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

        *state = AgentState::Arrived;

        drop(city_lock);
        city.force_unlock_for_main();
        return ThreadSignal::Yield;
    }

    match *state {
        AgentState::Arrived => ThreadSignal::Exit,
        _ => vehicle_logic(
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
            } else if pos.x < dest.x {
                pos.x += 1;
            }
            ThreadSignal::Yield
        }
        AgentState::WaitingForBridge => {
            let city_lock = match city.try_enter() {
                Some(lock) => lock,
                None => return ThreadSignal::Block,
            };

            let bridge = city_lock.get_bridge(3).expect("Puente 3 no encontrado");
            let can_cross = bridge.boat_request_pass();

            if can_cross {
                println!("[Boat-{}] ⛵ Puente levadizo levantado, pasando", id);
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
            }

            drop(city_lock);
            city.force_unlock_for_main(); // ✅ CAMBIADO
            return ThreadSignal::Yield; // ✅ CAMBIADO
        }
        AgentState::CrossingBridge => {
            *crossing_steps += 1;
            if *crossing_steps >= 5 {
                pos.x += 1;
                println!("[Boat-{}] ⛵ Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Traveling;

                let city_lock = match city.try_enter() {
                    Some(lock) => lock,
                    None => return ThreadSignal::Yield,
                };

                let bridge = city_lock.get_bridge(3).expect("Puente 3 no encontrado");
                bridge.boat_exit();

                drop(city_lock);
                city.force_unlock_for_main();
                return ThreadSignal::Yield;
            }
            ThreadSignal::Yield
        }
        AgentState::Arrived => ThreadSignal::Exit,
    }
}

// CORRECCIÓN: Función añadida de nuevo
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
    let row = rng.random_range(0..layout.grid_rows);
    let col = if rng.random_bool(0.5) {
        rng.random_range(0..layout.river_column)
    } else {
        rng.random_range((layout.river_column + 1)..layout.grid_cols)
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

fn random_supply_kind(rng: &mut impl Rng) -> SupplyKind {
    if rng.random_bool(0.5) {
        SupplyKind::Radioactive
    } else {
        SupplyKind::Water
    }
fn mainTRUE() {
    threadcity::run_simulation();
}
