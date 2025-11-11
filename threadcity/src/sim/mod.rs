// threadcity/src/sim/mod.rs
// Sistema de simulacion de ThreadCity
// REFACTORIZADO: Usa MyMutex en lugar de std::sync::Mutex

use crate::model::*;
use crate::{AgentInfo, AgentType};
use mypthreads::thread::ThreadId;
use rand::{rng, seq::IndexedRandom, Rng};
use std::collections::HashMap;
use mypthreads::sync::Shared;
use crate::tc_log; // <--- NUEVO

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
    pub agents: HashMap<ThreadId, AgentInfo>,
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
                next_vehicle_spawn_ms: 1000,
                next_boat_spawn_ms: 5000,
            },
            agents: HashMap::new(),
        }
    }

    pub fn add_river(&mut self) { self.river = Some(River::default()); }
    pub fn add_bridge(&mut self, bridge: Bridge) { self.bridges.push(bridge); }
    pub fn add_commerce(&mut self, id: u32, loc: (u32, u32)) { self.commerces.push(Commerce::new(id, Coord::new(loc.0, loc.1))); }

    pub fn add_nuclear_plant(
        &mut self,
        id: u32,
        loc: (u32, u32),
        requires: Vec<SupplySpec>,
        policy: DeadlinePolicy,
    ) {
        self.plants.push(NuclearPlant::new(id, Coord::new(loc.0, loc.1), requires, policy));
    }

    pub fn update(&mut self, dt_ms: u64) {
        self.time_ms += dt_ms;
        for bridge in &mut self.bridges { bridge.update(self.time_ms); }
    }

    pub fn current_time(&self) -> u64 { self.time_ms }
    pub fn get_bridge(&self, id: u32) -> Option<&Bridge> { self.bridges.iter().find(|b| b.id == id) }
    pub fn find_plant_at(&mut self, coord: Coord) -> Option<&mut NuclearPlant> {
        self.plants.iter_mut().find(|p| p.loc.x == coord.x && p.loc.y == coord.y)
    }

    /// Verifica deadlines de las plantas. Si una falla, imprime un mensaje y la reinicia.
    pub fn check_plant_deadlines(&mut self) {
        for plant in &mut self.plants {
            let _ = plant.tick_emergency(self.time_ms);

            let requires = plant.requires.clone();
            for supply in &requires {
                let last_delivery = plant.get_last_delivery_time(&supply.kind);
                let deadline = last_delivery + supply.deadline_ms;
                let fail_time = deadline + plant.deadline_policy.max_lateness_ms;

                let risk_threshold = 0.8;
                let risk_time = last_delivery + ((deadline - last_delivery) as f64 * risk_threshold) as u64;

                if self.time_ms > fail_time {
                    plant.status = PlantStatus::Exploded;
                    tc_log!(
                        "\n☢️☢️☢️ ¡EXPLOSIÓN! Planta {} falló por falta de {:?} en tiempo {}ms (Límite era: {}ms)",
                        plant.id, supply.kind, self.time_ms, fail_time
                    );
                    plant.reset(self.time_ms);
                    break;
                } else if self.time_ms > risk_time && plant.status == PlantStatus::Ok {
                    plant.status = PlantStatus::AtRisk;
                    tc_log!(
                        "\n⚠️⚠️⚠️ ¡ALERTA! Planta {} en riesgo por {:?}. Tiempo: {}ms (Límite: {}ms)",
                        plant.id, supply.kind, self.time_ms, deadline
                    );
                }
            }
        }
    }

    pub fn update_spawner(&mut self) -> Vec<AgentType> {
        let mut new_agents = Vec::new();
        let mut rng = rand::thread_rng();

        if self.time_ms >= self.spawner.next_vehicle_spawn_ms {
            let agent_type = if rng.gen_bool(0.1) { AgentType::Ambulance } else { AgentType::Car };
            new_agents.push(agent_type);

            let next_spawn_in = rng.gen_range(1000..4000);
            self.spawner.next_vehicle_spawn_ms = self.time_ms + next_spawn_in;
            tc_log!("[Spawner] Próximo vehículo en {}ms (Tiempo: {})", next_spawn_in, self.spawner.next_vehicle_spawn_ms);
        }

        if self.time_ms >= self.spawner.next_boat_spawn_ms {
            new_agents.push(AgentType::Boat);
            let next_spawn_in = rng.gen_range(15000..30000);
            self.spawner.next_boat_spawn_ms = self.time_ms + next_spawn_in;
            tc_log!("[Spawner] Próximo barco en {}ms (Tiempo: {})", next_spawn_in, self.spawner.next_boat_spawn_ms);
        }

        new_agents
    }
}

pub type SharedCity = Shared<City>;
pub fn create_shared_city(city: City) -> SharedCity { mypthreads::sync::shared(city) }
