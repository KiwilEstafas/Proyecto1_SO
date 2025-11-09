// Sistema de simulacion de ThreadCity

use crate::model::*;
use std::sync::{Arc, Mutex};
use rand::{rng, seq::IndexedRandom, Rng};
use crate::AgentType;

#[derive(Debug, Clone)]
pub struct Spawner {
    pub next_vehicle_spawn_ms: u64,
    pub next_boat_spawn_ms: u64,
}

pub struct City {
    pub time_ms: u64,
    pub grid: Grid,
    pub river: Option<River>,
    pub bridges: Vec<Bridge>,
    pub commerces: Vec<Commerce>,
    pub plants: Vec<NuclearPlant>,
    pub spawner: Spawner,
}

impl City {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            time_ms: 0,
            grid: Grid::new(rows, cols),
            river: None,
            bridges: Vec::new(),
            commerces: Vec::new(),
            plants: Vec::new(),
            spawner: Spawner {
                next_vehicle_spawn_ms: 1000, //EJEMPLO
                next_boat_spawn_ms: 5000,    //EJEMPLO
            },
        }
    }

    pub fn add_river(&mut self) {
        self.river = Some(River::default());
    }

    pub fn add_bridge(&mut self, bridge: Bridge) {
        self.bridges.push(bridge);
    }

    pub fn add_commerce(&mut self, id: u32, loc: (u32, u32)) {
        self.commerces
            .push(Commerce::new(id, Coord::new(loc.0, loc.1)));
    }

    pub fn add_nuclear_plant(
        &mut self,
        id: u32,
        loc: (u32, u32),
        requires: Vec<SupplySpec>,
        policy: DeadlinePolicy,
    ) {
        self.plants.push(NuclearPlant::new(
            id,
            Coord::new(loc.0, loc.1),
            requires,
            policy,
        ));
    }

    pub fn update(&mut self, dt_ms: u64) {
        self.time_ms += dt_ms;

        // Actualizar puentes (semáforos)
        for bridge in &mut self.bridges {
            bridge.update(self.time_ms);
        }
    }

    pub fn current_time(&self) -> u64 {
        self.time_ms
    }

    /// Encuentra un puente por ID
    pub fn get_bridge(&self, id: u32) -> Option<&Bridge> {
        self.bridges.iter().find(|b| b.id == id)
    }

    /// Encuentra una planta por coordenadas
    pub fn find_plant_at(&mut self, coord: Coord) -> Option<&mut NuclearPlant> {
        self.plants
            .iter_mut()
            .find(|p| p.loc.x == coord.x && p.loc.y == coord.y)
    }

    /// Verifica deadlines de las plantas. Si una falla, imprime un mensaje y la reinicia.
    pub fn check_plant_deadlines(&mut self) {
        // Ya no necesita devolver un Vec

        // Usamos un bucle `for` normal porque necesitamos modificar `plant`.
        for plant in &mut self.plants {
            // Si una planta ya explotó en este mismo ciclo de tiempo, no la revisamos de nuevo.
            // La reiniciaremos y volverá a estar 'Ok' para el próximo ciclo.
            if plant.status == PlantStatus::Exploded {
                continue;
            }

            // Hacemos una copia de los requerimientos para evitar problemas de borrowing.
            let requires = plant.requires.clone();

            for supply in &requires {
                let last_delivery = plant.get_last_delivery_time(&supply.kind);
                let deadline = last_delivery + supply.deadline_ms;
                let fail_time = deadline + plant.deadline_policy.max_lateness_ms;

                if self.time_ms > fail_time {
                    // Cambiamos el estado para registrar la explosión.
                    plant.status = PlantStatus::Exploded;

                    // Imprimimos un mensaje claro sobre el evento.
                    println!(
                    "\n☢️☢️☢️ ¡EXPLOSIÓN! Planta {} falló por falta de {:?} en tiempo {}ms (Límite era: {}ms)",
                    plant.id, supply.kind, self.time_ms, fail_time
                );

                    // ¡La nueva lógica! Inmediatamente reiniciamos la planta.
                    plant.reset(self.time_ms);

                    // Usamos `break` para salir del bucle de suministros.
                    // Si la planta explotó por falta de agua, no tiene sentido
                    // revisar también si falló por material radioactivo en el mismo instante.
                    // Una vez que explota, explota.
                    break;
                }
            }
        }

        // Ya no devolvemos `failures`.
    }

    pub fn update_spawner(&mut self) -> Vec<AgentType> {
        let mut new_agents = Vec::new();
        let mut rng = rand::thread_rng();

        // --- Lógica para Vehículos ---
        if self.time_ms >= self.spawner.next_vehicle_spawn_ms {
            // ¡Es hora de crear un vehículo!

            // Decidimos si es un carro o una ambulancia (ej. 10% de probabilidad de ambulancia)
            let agent_type = if rng.gen_bool(0.1) {
                AgentType::Ambulance
            } else {
                AgentType::Car
            };
            new_agents.push(agent_type);

            // Calculamos cuándo será el próximo spawn de vehículo.
            // Usamos un valor aleatorio para que no sea predecible (ej. entre 1 y 4 segundos)
            let next_spawn_in = rng.gen_range(1000..4000);
            self.spawner.next_vehicle_spawn_ms = self.time_ms + next_spawn_in;
            println!(
                "[Spawner] Próximo vehículo en {}ms (Tiempo: {})",
                next_spawn_in, self.spawner.next_vehicle_spawn_ms
            );
        }

        // --- Lógica para Barcos ---
        if self.time_ms >= self.spawner.next_boat_spawn_ms {
            new_agents.push(AgentType::Boat);
            let next_spawn_in = rng.gen_range(15000..30000); // Los barcos son menos frecuentes
            self.spawner.next_boat_spawn_ms = self.time_ms + next_spawn_in;
            println!(
                "[Spawner] Próximo barco en {}ms (Tiempo: {})",
                next_spawn_in, self.spawner.next_boat_spawn_ms
            );
        }

        new_agents
    }
}

/// Estructura compartida de la ciudad para hilos
pub type SharedCity = Arc<Mutex<City>>;

pub fn create_shared_city(city: City) -> SharedCity {
    Arc::new(Mutex::new(city))
}
