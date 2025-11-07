// Sistema de spawning compatible con mypthreads v2

use crate::agents::{Car, Ambulance, Boat, CargoTruck, AgentDowncast};
use crate::model::{SupplyKind, NuclearPlant};
use rand::Rng;
use rand_distr::{Distribution, Poisson};

pub struct CityLayout {
    pub grid_rows: u32,
    pub grid_cols: u32,
    pub river_column: u32,
    pub bridge1_row: u32,
    pub bridge2_row: u32,
    pub bridge3_row: u32,
}

impl Default for CityLayout {
    fn default() -> Self {
        Self {
            grid_rows: 5,
            grid_cols: 5,
            river_column: 2,
            bridge1_row: 1,
            bridge2_row: 2,
            bridge3_row: 3,
        }
    }
}

pub struct VehicleSpawner {
    rng: rand::rngs::ThreadRng,
    poisson: Poisson<f64>,
    pub vehicles_spawned: u32,
    pub boats_spawned: u32,
    pub trucks_spawned: u32,
    next_vehicle_id: u32,
    next_boat_id: u32,
    next_truck_id: u32,
    last_boat_spawn_tick: u32,
    last_truck_spawn_tick: u32,
}

impl VehicleSpawner {
    pub fn new(mean_spawn_rate: f64) -> Self {
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

    fn random_position(&mut self, layout: &CityLayout, avoid_river: bool) -> (u32, u32) {
        let row = self.rng.random_range(0..layout.grid_rows);
        let col = if avoid_river {
            if self.rng.random_bool(0.5) {
                self.rng.random_range(0..layout.river_column)
            } else {
                self.rng.random_range((layout.river_column + 1)..layout.grid_cols)
            }
        } else {
            self.rng.random_range(0..layout.grid_cols)
        };
        (row, col)
    }

    fn random_destination(
        &mut self,
        origin: (u32, u32),
        layout: &CityLayout,
        avoid_river: bool,
    ) -> (u32, u32) {
        let mut dest;
        loop {
            dest = self.random_position(layout, avoid_river);
            if dest != origin {
                break;
            }
        }
        dest
    }

    pub fn should_spawn(&mut self) -> bool {
        let sample = self.poisson.sample(&mut self.rng);
        sample > 0.5
    }

    pub fn should_spawn_boat(&mut self, current_tick: u32) -> bool {
        current_tick - self.last_boat_spawn_tick >= 100
    }

    pub fn should_spawn_truck(&mut self, current_tick: u32) -> bool {
        current_tick - self.last_truck_spawn_tick >= 50
    }

    pub fn spawn_vehicle(
        &mut self,
        layout: &CityLayout,
    ) -> Box<dyn AgentDowncast + Send> {
        let origin = self.random_position(layout, true);
        let destination = self.random_destination(origin, layout, true);

        let vehicle_type = self.rng.random_range(0..10);
        let id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        self.vehicles_spawned += 1;

        if vehicle_type == 0 {
            println!("🚑 Spawning Ambulancia-{} en {:?} -> {:?}", id, origin, destination);
            Box::new(Ambulance::new(id, origin, destination))
        } else {
            println!("🚗 Spawning Carro-{} en {:?} -> {:?}", id, origin, destination);
            Box::new(Car::new(id, origin, destination))
        }
    }

    pub fn spawn_boat(
        &mut self,
        layout: &CityLayout,
        current_tick: u32,
    ) -> Box<dyn AgentDowncast + Send> {
        let id = self.next_boat_id;
        self.next_boat_id += 1;
        self.boats_spawned += 1;
        self.last_boat_spawn_tick = current_tick;

        let start_row = self.rng.random_range(layout.bridge1_row..=layout.bridge2_row);
        let origin = (start_row, layout.river_column);
        let dest_row = layout.bridge3_row + 1;
        let destination = (dest_row, layout.river_column);

        println!("⛵ Spawning Barco-{} en {:?} -> {:?}", id, origin, destination);
        Box::new(Boat::new(id, origin, destination))
    }

    pub fn spawn_cargo_truck(
        &mut self,
        layout: &CityLayout,
        current_tick: u32,
        plants: &[NuclearPlant],
    ) -> Box<dyn AgentDowncast + Send> {
        let id = self.next_truck_id;
        self.next_truck_id += 1;
        self.trucks_spawned += 1;
        self.last_truck_spawn_tick = current_tick;

        let plant_idx = self.rng.random_range(0..plants.len());
        let plant = &plants[plant_idx];
        let destination = (plant.loc.x, plant.loc.y);

        let supply_idx = self.rng.random_range(0..plant.requires.len());
        let cargo = plant.requires[supply_idx].kind;

        // Origen en el lado opuesto al río
        let origin = if destination.1 < layout.river_column {
            let row = self.rng.random_range(0..layout.grid_rows);
            let col = self.rng.random_range((layout.river_column + 1)..layout.grid_cols);
            (row, col)
        } else {
            let row = self.rng.random_range(0..layout.grid_rows);
            let col = self.rng.random_range(0..layout.river_column);
            (row, col)
        };

        println!(
            "🚚 Spawning CargoTruck-{} ({:?}) en {:?} -> Planta {} {:?}",
            id, cargo, origin, plant.id, destination
        );
        Box::new(CargoTruck::new(id, origin, destination, cargo))
    }
}
