// Sistema de simulacion de ThreadCity

use crate::model::*;
use std::sync::{Arc, Mutex};

pub struct City {
    pub time_ms: u64,
    pub grid: Grid,
    pub river: Option<River>,
    pub bridges: Vec<Bridge>,
    pub commerces: Vec<Commerce>,
    pub plants: Vec<NuclearPlant>,
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
        }
    }
    
    pub fn add_river(&mut self) {
        self.river = Some(River::default());
    }
    
    pub fn add_bridge(&mut self, bridge: Bridge) {
        self.bridges.push(bridge);
    }
    
    pub fn add_commerce(&mut self, id: u32, loc: (u32, u32)) {
        self.commerces.push(Commerce::new(id, Coord::new(loc.0, loc.1)));
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
        self.plants.iter_mut().find(|p| p.loc.x == coord.x && p.loc.y == coord.y)
    }
    
    /// Verifica deadlines de las plantas
    pub fn check_plant_deadlines(&mut self) -> Vec<(u32, SupplyKind)> {
        let mut failures = Vec::new();
        
        for plant in &mut self.plants {
            if plant.status == PlantStatus::Exploded {
                continue;
            }
            
            for supply in &plant.requires {
                let last_delivery = plant.get_last_delivery_time(&supply.kind);
                let deadline = last_delivery + supply.deadline_ms;
                let fail_time = deadline + plant.deadline_policy.max_lateness_ms;
                
                if self.time_ms > fail_time {
                    plant.status = PlantStatus::Exploded;
                    failures.push((plant.id, supply.kind));
                    println!("\n☢️☢️☢️ ¡EXPLOSIÓN! Planta {} falló por {:?}", plant.id, supply.kind);
                    println!("    Tiempo: {}ms, Límite: {}ms", self.time_ms, fail_time);
                }
            }
        }
        
        failures
    }
}

/// Estructura compartida de la ciudad para hilos
pub type SharedCity = Arc<Mutex<City>>;

pub fn create_shared_city(city: City) -> SharedCity {
    Arc::new(Mutex::new(city))
}