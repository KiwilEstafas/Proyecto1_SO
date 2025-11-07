// Simulación principal integrada con mypthreads v2

use crate::agents::agent_controller::{AgentContext, AgentPhase};
use crate::model::{Bridge, Coord, DeadlinePolicy, NuclearPlant, PlantStatus, SupplyKind, SupplySpec, TrafficDirection};
use crate::sim::spawner::{CityLayout, VehicleSpawner};
use mypthreads::mypthreads_api::*;
use mypthreads::signals::ThreadSignal;
use mypthreads::thread::ThreadId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ThreadCitySimulation {
    layout: CityLayout,
    bridges: Vec<Arc<Bridge>>,
    plants: Arc<Mutex<Vec<NuclearPlant>>>,
    spawner: VehicleSpawner,
    truck_threads: HashMap<ThreadId, SupplyKind>,
    tick: u32,
}

impl ThreadCitySimulation {
    pub fn new() -> Self {
        let layout = CityLayout::default();

        // Crear puentes
        let bridges = vec![
            Arc::new(Bridge::new_traffic_light(1, layout.bridge1_row, 5000)),
            Arc::new(Bridge::new_yield(2, layout.bridge2_row, TrafficDirection::WestToEast)),
            Arc::new(Bridge::new_drawbridge(3, layout.bridge3_row)),
        ];

        // Crear plantas nucleares
        let plant1 = NuclearPlant::new(
            1,
            Coord::new(0, 1), // Zona oeste
            vec![
                SupplySpec {
                    kind: SupplyKind::Radioactive,
                    deadline_ms: 10_000,
                    period_ms: 20_000,
                },
                SupplySpec {
                    kind: SupplyKind::Water,
                    deadline_ms: 8_000,
                    period_ms: 16_000,
                },
            ],
            DeadlinePolicy {
                max_lateness_ms: 2_000,
            },
        );

        let plant2 = NuclearPlant::new(
            2,
            Coord::new(1, 3), // Zona este
            vec![
                SupplySpec {
                    kind: SupplyKind::Radioactive,
                    deadline_ms: 12_000,
                    period_ms: 24_000,
                },
                SupplySpec {
                    kind: SupplyKind::Water,
                    deadline_ms: 10_000,
                    period_ms: 20_000,
                },
            ],
            DeadlinePolicy {
                max_lateness_ms: 2_500,
            },
        );

        let plants = Arc::new(Mutex::new(vec![plant1, plant2]));

        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║           ThreadCity con MyPthreads v2                     ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("🌊 Río en columna {}", layout.river_column);
        println!("🌉 Puente 1 (Semáforo) en fila {}", layout.bridge1_row);
        println!("🌉 Puente 2 (Ceda) en fila {}", layout.bridge2_row);
        println!("🌉 Puente 3 (Levadizo) en fila {}", layout.bridge3_row);
        println!("☢️  Planta 1 en (0, 1) - Zona Oeste");
        println!("☢️  Planta 2 en (1, 3) - Zona Este\n");

        Self {
            layout,
            bridges,
            plants,
            spawner: VehicleSpawner::new(0.3),
            truck_threads: HashMap::new(),
            tick: 0,
        }
    }

    pub fn run(&mut self, max_ticks: u32, min_vehicles: u32) {
        println!("🚀 Iniciando simulación (máx {} ticks, mín {} vehículos)\n", max_ticks, min_vehicles);

        while self.tick < max_ticks && self.spawner.vehicles_spawned < min_vehicles {
            // Spawning de vehículos
            if self.spawner.vehicles_spawned < min_vehicles && self.spawner.should_spawn() {
                let agent = self.spawner.spawn_vehicle(&self.layout);
                self.spawn_agent(agent, SchedulerParams::Lottery { tickets: 10 });
            }

            // Spawning de barcos
            if self.spawner.should_spawn_boat(self.tick) {
                let boat = self.spawner.spawn_boat(&self.layout, self.tick);
                self.spawn_agent(boat, SchedulerParams::RoundRobin);
            }

            // Spawning de camiones
            if self.spawner.should_spawn_truck(self.tick) {
                let plants = self.plants.lock().unwrap();
                if !plants.is_empty() {
                    let truck = self.spawner.spawn_cargo_truck(&self.layout, self.tick, &plants);
                    drop(plants);
                    self.spawn_agent(truck, SchedulerParams::RealTime { deadline: self.tick as u64 + 10_000 });
                }
            }

            // Avanzar un ciclo del runtime
            run_simulation(1);

            // Actualizar estado de puentes (semáforos)
            for bridge in self.bridges.iter() {
                let bridge_mut = Arc::get_mut(&mut bridge.clone()).unwrap();
                bridge_mut.step(100);
            }

            // Verificar estado de plantas nucleares
            self.check_plants_status();

            // Log de progreso
            if self.tick % 50 == 0 {
                println!(
                    "⏱️  Tick: {}, Vehículos: {}, Camiones: {}, Barcos: {}",
                    self.tick,
                    self.spawner.vehicles_spawned,
                    self.spawner.trucks_spawned,
                    self.spawner.boats_spawned
                );
            }

            self.tick += 1;
        }

        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║           Simulación Finalizada                           ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Ticks totales: {:>43} ║", self.tick);
        println!("║ Vehículos generados: {:>37} ║", self.spawner.vehicles_spawned);
        println!("║ Camiones generados: {:>38} ║", self.spawner.trucks_spawned);
        println!("║ Barcos generados: {:>40} ║", self.spawner.boats_spawned);
        println!("╚════════════════════════════════════════════════════════════╝\n");
    }

    fn spawn_agent(&mut self, agent: Box<dyn crate::agents::AgentDowncast + Send>, params: SchedulerParams) {
        let agent_id = agent.id();
        let destination = if let Some(car) = agent.as_any().downcast_ref::<crate::agents::Car>() {
            Coord::new(car.inner.destination.x, car.inner.destination.y)
        } else if let Some(amb) = agent.as_any().downcast_ref::<crate::agents::Ambulance>() {
            Coord::new(amb.inner.destination.x, amb.inner.destination.y)
        } else if let Some(truck) = agent.as_any().downcast_ref::<crate::agents::CargoTruck>() {
            Coord::new(truck.inner.destination.x, truck.inner.destination.y)
        } else if let Some(boat) = agent.as_any().downcast_ref::<crate::agents::Boat>() {
            Coord::new(boat.inner.destination.x, boat.inner.destination.y)
        } else {
            Coord::new(4, 4)
        };

        let context = Arc::new(Mutex::new(AgentContext::new(
            agent,
            destination,
            self.layout.river_column,
            self.bridges.clone(),
        )));

        let context_clone = context.clone();
        let agent_name = format!("Agent-{}", agent_id);

        let entry = Box::new(move |_tid: ThreadId| -> ThreadSignal {
            let mut ctx = context_clone.lock().unwrap();
            ctx.step()
        });

        let tid = my_thread_create(&agent_name, params, entry);
        println!("✅ Hilo creado: {} (tid: {})", agent_name, tid);
    }

    fn check_plants_status(&mut self) {
        let mut plants = self.plants.lock().unwrap();
        // TODO: Implementar lógica de verificación de deadlines
        // Por ahora solo un placeholder
        for plant in plants.iter_mut() {
            if plant.status == PlantStatus::Ok {
                // Verificar si algún deadline está cerca de vencerse
                // Activar urgencia en camiones si es necesario
            }
        }
    }
}

pub fn run_threadcity_simulation() {
    let mut sim = ThreadCitySimulation::new();
    sim.run(1000, 25);
}
