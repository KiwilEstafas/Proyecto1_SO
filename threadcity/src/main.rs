// ThreadCity - Simulación con hilos preemptivos usando mypthreads

use mypthreads::mypthreads_api::RUNTIME;
use mypthreads::mypthreads_api::*;
use mypthreads::runtime::ThreadRuntimeV2;
use mypthreads::signals::ThreadSignal;
use mypthreads::thread::ThreadId;
use rand::{rng, seq::IndexedRandom, Rng};
use std::sync::{Arc, Mutex};
use threadcity::*;

/// Estado de un agente (vehículo/barco) en la simulación
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentState {
    Moving,
    WaitingForBridge,
    CrossingBridge,
    Arrived,
}

/// Tipo de agente
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentType {
    Car,
    Ambulance,
    Boat,
    CargoTruck(SupplyKind),
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           ThreadCity - Simulación Preemptiva              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Crear ciudad
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);

    // Contadores de agentes creados
    let cars_created = Arc::new(Mutex::new(0u32));
    let ambulances_created = Arc::new(Mutex::new(0u32));
    let boats_created = Arc::new(Mutex::new(0u32));
    let trucks_created = Arc::new(Mutex::new(0u32));

    println!("Iniciando simulación...\n");

    // Crear algunos carros
    for i in 0..5 {
        spawn_car(i + 1, &layout, &shared_city);
        *cars_created.lock().unwrap() += 1;
    }

    // Crear ambulancias
    for i in 0..2 {
        spawn_ambulance(i + 100, &layout, &shared_city);
        *ambulances_created.lock().unwrap() += 1;
    }

    // Crear camiones de carga para las plantas
    println!("Creando camiones de carga aleatorios...");
    for i in 0..4 {
        // La nueva función solo necesita el ID, el layout y la ciudad.
        // El resto (origen, destino, carga) se decide dentro de la función.
        spawn_cargo_truck(200 + i, &layout, &shared_city);
    }
    *trucks_created.lock().unwrap() = 4;

    // Crear un barco
    spawn_boat(300, &layout, &shared_city);
    *boats_created.lock().unwrap() += 1;

    println!("Agentes creados:");
    println!("  🚗 Carros: {}", *cars_created.lock().unwrap());
    println!("  🚑 Ambulancias: {}", *ambulances_created.lock().unwrap());
    println!("  🚚 Camiones: {}", *trucks_created.lock().unwrap());
    println!("  ⛵ Barcos: {}", *boats_created.lock().unwrap());
    println!();

    // Ejecutar simulación
    const SIMULATION_STEPS: u32 = 200; // ¿Cuántos "pasos" durará la simulación?
    const TIME_PER_STEP_MS: u64 = 500; // ¿Cuántos milisegundos del "mundo" avanzan en cada paso?
    const SCHEDULER_CYCLES_PER_STEP: usize = 10; // ¿Cuántos ciclos de CPU damos a los hilos en cada paso?
    println!(
        "Iniciando simulación... Pasos: {}, Tiempo/Paso: {}ms\n",
        SIMULATION_STEPS, TIME_PER_STEP_MS
    );

    for step in 0..SIMULATION_STEPS {
        // --- Parte A: Actualizar el estado del MUNDO ---
        {
            // Usamos un bloque para que el Mutex se libere lo antes posible
            let mut city_lock = shared_city.lock().unwrap();

            // 1. Avanzar el reloj global de la ciudad
            city_lock.update(TIME_PER_STEP_MS);

            // 2. Revisar si alguna planta explotó con el nuevo tiempo
            let failures = city_lock.check_plant_deadlines();
            if !failures.is_empty() {
                println!("☢️  ¡UNA PLANTA HA EXPLOTADO! Finalizando simulación.");
                break; // Termina el bucle principal si hay una explosión
            }

            // Imprimimos el estado actual para poder depurar
            println!(
                "\n--- [Paso de Simulación {} | Tiempo Mundial: {}ms] ---",
                step,
                city_lock.current_time()
            );
        } // El Mutex de la ciudad se libera aquí, permitiendo que los hilos accedan a ella.

        // --- Parte B: Permitir que los AGENTES actúen ---

        // 3. Desbloquear a todos los hilos que estaban esperando
        //    (Necesitarás añadir esta función a tu Runtime, ver explicación más abajo)
        // RUNTIME.lock().unwrap().unblock_all_threads();

        // 4. Ejecutar el planificador de hilos por unos cuantos ciclos
        //    Esto les da a los carros, barcos, etc., la oportunidad de moverse y reaccionar.
        RUNTIME.lock().unwrap().unblock_all_threads();
        RUNTIME.lock().unwrap().run(SCHEDULER_CYCLES_PER_STEP);

        // Opcional: una pequeña pausa real para que la salida en consola no sea abrumadora
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              Simulación Finalizada                        ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Carros: {:>51} ║", *cars_created.lock().unwrap());
    println!(
        "║ Ambulancias: {:>47} ║",
        *ambulances_created.lock().unwrap()
    );
    println!("║ Camiones: {:>49} ║", *trucks_created.lock().unwrap());
    println!("║ Barcos: {:>51} ║", *boats_created.lock().unwrap());
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

/// Crea un carro normal
fn spawn_car(id: u32, layout: &CityLayout, city: &SharedCity) {
    let mut rng = rand::rng();

    // Origen y destino aleatorios (evitando columna del río)
    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);

    let city_clone = Arc::clone(city);
    let layout_clone = layout.clone();

    let mut pos = origin;
    let mut state = AgentState::Moving;
    let mut crossing_steps = 0u32;

    println!("🚗 Carro-{} creado: {:?} -> {:?}", id, origin, dest);

    my_thread_create(
        &format!("Car-{}", id),
        SchedulerParams::Lottery { tickets: 10 },
        Box::new(move |tid| {
            vehicle_logic(
                tid,
                id,
                AgentType::Car,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
}

/// Crea una ambulancia (prioridad alta)
fn spawn_ambulance(id: u32, layout: &CityLayout, city: &SharedCity) {
    let mut rng = rand::rng();

    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);

    let city_clone = Arc::clone(city);
    let layout_clone = layout.clone();

    let mut pos = origin;
    let mut state = AgentState::Moving;
    let mut crossing_steps = 0u32;

    println!("🚑 Ambulancia-{} creada: {:?} -> {:?}", id, origin, dest);

    my_thread_create(
        &format!("Ambulance-{}", id),
        SchedulerParams::Lottery { tickets: 100 }, // Más tickets = más prioridad
        Box::new(move |tid| {
            vehicle_logic(
                tid,
                id,
                AgentType::Ambulance,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
}

fn random_supply_kind(rng: &mut rand::prelude::ThreadRng) -> SupplyKind {
    if rng.random_bool(0.5) {
        SupplyKind::Radioactive
    } else {
        SupplyKind::Water
    }
}

/// Crea un camión de carga para planta nuclear
/// Crea un camión de carga con origen, destino (planta) y carga aleatorios.
fn spawn_cargo_truck(id: u32, layout: &CityLayout, city: &SharedCity) {
    // Obtenemos el generador de números aleatorios una vez
    let mut rng = rand::thread_rng();

    // 1. Generar un origen y una carga aleatorios
    let origin = random_position(&mut rng, layout);
    let cargo = random_supply_kind(&mut rng);

    // Variables que obtendremos de la ciudad
    let destination: Coord;
    let deadline: u64;

    // 2. Bloquear la ciudad UNA SOLA VEZ para obtener la información de la planta
    {
        // Usamos un bloque para que el Mutex se libere lo antes posible
        let city_lock = city.lock().unwrap();

        // Elegimos una planta al azar de la lista
        // .expect es para simplificar; en un programa real manejaríamos el caso de que no haya plantas
        let plant = city_lock
            .plants
            .choose(&mut rng)
            .expect("No se encontraron plantas en la ciudad")
            .clone();

        // El destino es la ubicación de la planta elegida
        destination = plant.loc;

        // Calculamos el deadline usando la información de ESA planta
        let supply_spec = plant
            .requires
            .iter()
            .find(|s| s.kind == cargo)
            .expect("La planta no requiere este suministro");

        deadline = city_lock.current_time() + supply_spec.deadline_ms;
    } // El Mutex se libera aquí

    // Preparamos los clones para el hilo ANTES de mover las variables
    let city_clone = Arc::clone(city);
    let layout_clone = layout.clone();
    let mut pos = origin; // El estado inicial se basa en el origen aleatorio
    let mut state = AgentState::Moving;
    let mut crossing_steps = 0u32;

    println!(
        "🚚 CargoTruck-{} ({:?}): {:?} -> {:?}, deadline: {}ms",
        id, cargo, origin, destination, deadline
    );

    my_thread_create(
        &format!("Truck-{}", id),
        SchedulerParams::RealTime { deadline },
        Box::new(move |tid| {
            cargo_truck_logic(
                tid,
                id,
                cargo,
                &mut pos,
                destination, // Usamos el destino aleatorio
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
}

/// Crea un barco
fn spawn_boat(id: u32, layout: &CityLayout, city: &SharedCity) {
    let city_clone = Arc::clone(city);
    let layout_clone = layout.clone();

    // Los barcos navegan verticalmente en el río
    let origin = Coord::new(layout.bridge1_row, layout.river_column);
    let dest = Coord::new(layout.bridge3_row + 1, layout.river_column);

    let mut pos = origin;
    let mut state = AgentState::Moving;
    let mut crossing_steps = 0u32;

    println!("⛵ Barco-{} creado: {:?} -> {:?}", id, origin, dest);

    my_thread_create(
        &format!("Boat-{}", id),
        SchedulerParams::RoundRobin,
        Box::new(move |tid| {
            boat_logic(
                tid,
                id,
                &mut pos,
                dest,
                &mut state,
                &mut crossing_steps,
                &city_clone,
                &layout_clone,
            )
        }),
    );
}

/// Lógica general de vehículos terrestres
fn vehicle_logic(
    tid: ThreadId,
    id: u32,
    agent_type: AgentType,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Moving => {
            // Verificar si llegó
            if pos.x == dest.x && pos.y == dest.y {
                println!("[{}] ✅ Llegó a destino {:?}", id, dest);
                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }

            // Verificar si necesita cruzar el río
            let needs_bridge = (pos.y < layout.river_column && dest.y > layout.river_column)
                || (pos.y > layout.river_column && dest.y < layout.river_column);

            let at_bridge_entrance = (pos.y == layout.river_column - 1
                && dest.y > layout.river_column)
                || (pos.y == layout.river_column + 1 && dest.y < layout.river_column);

            if needs_bridge && at_bridge_entrance {
                println!("[{}] 🚦 En entrada de puente", id);
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }

            // Moverse hacia el destino
            move_towards(pos, dest, layout);
            ThreadSignal::Yield
        }

        AgentState::WaitingForBridge => {
            let city_lock = city.lock().unwrap();
            let bridge_id = nearest_bridge(layout, pos.x);
            let bridge = city_lock
                .get_bridge(bridge_id)
                .expect("Puente no encontrado");

            let priority = match agent_type {
                AgentType::Ambulance => 10,
                _ => 0,
            };

            let direction = if pos.y < layout.river_column {
                TrafficDirection::NorthToSouth
            } else {
                TrafficDirection::SouthToNorth
            };

            // Ambulancias pasan sin esperar
            if agent_type == AgentType::Ambulance {
                println!("[{}] 🚑 AMBULANCIA pasando directamente", id);
                *state = AgentState::CrossingBridge;
                *crossing_steps = 0;
                drop(city_lock);
                return ThreadSignal::Yield;
            }

            if bridge.try_cross(tid, priority, direction) {
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
                // Terminar de cruzar
                if pos.y < layout.river_column {
                    pos.y = layout.river_column + 1;
                } else {
                    pos.y = layout.river_column - 1;
                }

                let city_lock = city.lock().unwrap();
                let bridge_id = nearest_bridge(layout, pos.x);
                let bridge = city_lock
                    .get_bridge(bridge_id)
                    .expect("Puente no encontrado");

                if agent_type != AgentType::Ambulance {
                    bridge.exit_bridge(tid);
                }

                drop(city_lock);

                println!("[{}] Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Moving;
            }

            ThreadSignal::Yield
        }

        AgentState::Arrived => ThreadSignal::Exit,
    }
}

/// Lógica de camión de carga
fn cargo_truck_logic(
    tid: ThreadId,
    id: u32,
    cargo: SupplyKind,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Moving => {
            if pos.x == dest.x && pos.y == dest.y {
                // Entregar suministro
                let mut city_lock = city.lock().unwrap();
                let current_time = city_lock.current_time(); // Guardar el tiempo PRIMERO
                if let Some(plant) = city_lock.find_plant_at(dest) {
                    let supply = plant
                        .requires
                        .iter()
                        .find(|s| s.kind == cargo)
                        .expect("Suministro no requerido")
                        .clone();

                    plant.commit_delivery(supply, current_time); // Usar la variable guardada
                    println!(
                        "[Truck-{}] ✅ Entrega de {:?} a Planta en {:?}",
                        id, cargo, dest
                    );
                }
                drop(city_lock);

                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }

            // Similar a vehículos normales
            let needs_bridge = (pos.y < layout.river_column && dest.y > layout.river_column)
                || (pos.y > layout.river_column && dest.y < layout.river_column);

            let at_bridge_entrance = (pos.y == layout.river_column - 1
                && dest.y > layout.river_column)
                || (pos.y == layout.river_column + 1 && dest.y < layout.river_column);

            if needs_bridge && at_bridge_entrance {
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }

            move_towards(pos, dest, layout);
            ThreadSignal::Yield
        }

        AgentState::WaitingForBridge | AgentState::CrossingBridge => {
            // Usar la misma lógica que vehículos normales
            vehicle_logic(
                tid,
                id,
                AgentType::CargoTruck(cargo),
                pos,
                dest,
                state,
                crossing_steps,
                city,
                layout,
            )
        }

        AgentState::Arrived => ThreadSignal::Exit,
    }
}

/// Lógica de barco
fn boat_logic(
    tid: ThreadId,
    id: u32,
    pos: &mut Coord,
    dest: Coord,
    state: &mut AgentState,
    crossing_steps: &mut u32,
    city: &SharedCity,
    layout: &CityLayout,
) -> ThreadSignal {
    match *state {
        AgentState::Moving => {
            if pos.x == dest.x && pos.y == dest.y {
                println!("[Boat-{}] ✅ Llegó a destino {:?}", id, dest);
                *state = AgentState::Arrived;
                return ThreadSignal::Exit;
            }

            // Verificar si está en el puente 3
            if pos.x == layout.bridge3_row {
                *state = AgentState::WaitingForBridge;
                return ThreadSignal::Yield;
            }

            // Moverse verticalmente en el río
            if pos.x < dest.x {
                pos.x += 1;
            }

            ThreadSignal::Yield
        }

        AgentState::WaitingForBridge => {
            let city_lock = city.lock().unwrap();
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
                let city_lock = city.lock().unwrap();
                let bridge = city_lock.get_bridge(3).expect("Puente 3 no encontrado");
                bridge.boat_exit();
                drop(city_lock);

                pos.x += 1;
                println!("[Boat-{}] ⛵ Cruzó el puente, pos: {:?}", id, pos);
                *state = AgentState::Moving;
            }

            ThreadSignal::Yield
        }

        AgentState::Arrived => ThreadSignal::Exit,
    }
}

/// Mueve una posición hacia el destino
fn move_towards(pos: &mut Coord, dest: Coord, layout: &CityLayout) {
    // Evitar la columna del río
    if pos.y != layout.river_column {
        if pos.y < dest.y && pos.y + 1 != layout.river_column {
            pos.y += 1;
            return;
        } else if pos.y > dest.y && pos.y - 1 != layout.river_column {
            pos.y -= 1;
            return;
        }
    }

    // Mover verticalmente
    if pos.x < dest.x {
        pos.x += 1;
    } else if pos.x > dest.x {
        pos.x -= 1;
    }
}

/// Genera una posición aleatoria (evitando río)
fn random_position(rng: &mut rand::prelude::ThreadRng, layout: &CityLayout) -> Coord {
    let row = rng.random_range(0..layout.grid_rows);
    let col = if rng.random_bool(0.5) {
        rng.random_range(0..layout.river_column)
    } else {
        rng.random_range((layout.river_column + 1)..layout.grid_cols)
    };
    Coord::new(row, col)
}

/// Genera un destino aleatorio diferente al origen
fn random_destination(
    rng: &mut rand::prelude::ThreadRng,
    layout: &CityLayout,
    origin: Coord,
) -> Coord {
    loop {
        let dest = random_position(rng, layout);
        if dest.x != origin.x || dest.y != origin.y {
            return dest;
        }
    }
}
