// ciudad y avance de simulacion

mod spawner;    
mod simulation; 

pub use spawner::{VehicleSpawner, CityLayout};
pub use simulation::{ThreadCitySimulation, run_threadcity_simulation};
use std::thread;
use std::time::Duration;

use crate::model::*;
use crate::agents::{Agent, AgentDowncast, CargoTruck};

pub struct City {
    now_ms: u64,
    pub grid: Grid,
    pub river: Option<River>,
    pub bridges: Vec<Bridge>,
    pub commerces: Vec<Commerce>,
    pub plants: Vec<NuclearPlant>,
    pub agents: Vec<Box<dyn AgentDowncast + Send>>,
}

impl City {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            now_ms: 0,
            grid: Grid::new(rows, cols),
            river: None,
            bridges: vec![],
            commerces: vec![],
            plants: vec![],
            agents: vec![],
        }
    }

    pub fn now(&self) -> u64 { self.now_ms }

    pub fn add_river(&mut self) { self.river = Some(River::default()); }
    pub fn add_commerce(&mut self, id: u32, loc: (u32, u32)) {
        self.commerces.push(Commerce::new(id, Coord::new(loc.0, loc.1)));
    }
    pub fn add_nuclear_plant(&mut self, id: u32, loc: (u32,u32), requires: Vec<SupplySpec>, policy: DeadlinePolicy) {
        self.plants.push(NuclearPlant::new(id, Coord::new(loc.0, loc.1),requires, policy));
    }

    pub fn add_agent(&mut self, a: Box<dyn AgentDowncast + Send>) { self.agents.push(a); }

    pub fn step(&mut self, dt_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(dt_ms);
        for a in self.agents.iter_mut() {
            // como AgentDowncast:Agent, podemos llamar step
            a.step(dt_ms as u32);
        }

        if let Some(plant) = self.plants.get_mut(0) {
            for a in self.agents.iter() {
                if let Some(ct) = a.as_any().downcast_ref::<CargoTruck>() {
                    if ct.pos().x == 5 && ct.pos().y == 5 {
                        let spec = SupplySpec { kind: SupplyKind::Water, deadline_ms: 5_000, period_ms: 10_000 };
                        plant.commit_delivery(spec, self.now_ms);
                    }
                }
            }
        }
    }

    pub fn run_agents_in_threads_for_demo(&mut self, ticks: u32, sleep_ms: u64) {
        let mut handles = vec![];
        for boxed in self.agents.drain(..) {
            let mut a = boxed;
            let h = thread::spawn(move || {
                for _ in 0..ticks {
                    a.step(1000);
                    thread::sleep(Duration::from_millis(sleep_ms));
                }
                a
            });
            handles.push(h);
        }
        for h in handles {
            if let Ok(a) = h.join() {
                self.agents.push(a);
            }
        }
    }
}

