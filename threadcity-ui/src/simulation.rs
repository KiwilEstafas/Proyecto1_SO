// Bridge entre la simulación threadcity y la UI

use threadcity::*;
use mypthreads::mypthreads_api::*;
use mypthreads::signals::ThreadSignal;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualAgentType {
    Car,
    Ambulance,
    Truck,
    Boat,
}

#[derive(Debug, Clone)]
pub struct VisualAgent {
    pub id: u32,
    pub agent_type: VisualAgentType,
    pub pos: Coord,
    pub dest: Coord,
}

pub struct SimulationState {
    pub city: Arc<Mutex<City>>,
    pub layout: CityLayout,
    pub agents: Arc<Mutex<Vec<VisualAgent>>>,
    pub cycle: u32,
    pub deliveries: u32,
    pub total_agents: u32,
}

impl SimulationState {
    pub fn new() -> Self {
        let (city, layout) = create_city();
        let shared_city = create_shared_city(city);
        
        let agents = Arc::new(Mutex::new(Vec::new()));
        
        // Crear agentes iniciales
        let mut total = 0;
        
        // Carros
        for i in 1..=3 {
            spawn_visual_car(i, &layout, &shared_city, Arc::clone(&agents));
            total += 1;
        }
        
        // Ambulancias
        for i in 100..=101 {
            spawn_visual_ambulance(i, &layout, &shared_city, Arc::clone(&agents));
            total += 1;
        }
        
        // Camiones
        spawn_visual_truck(200, SupplyKind::Radioactive, (0, 0), (1, 0), &layout, &shared_city, Arc::clone(&agents));
        spawn_visual_truck(201, SupplyKind::Water, (0, 0), (1, 0), &layout, &shared_city, Arc::clone(&agents));
        spawn_visual_truck(202, SupplyKind::Radioactive, (0, 4), (2, 4), &layout, &shared_city, Arc::clone(&agents));
        spawn_visual_truck(203, SupplyKind::Water, (0, 4), (2, 4), &layout, &shared_city, Arc::clone(&agents));
        total += 4;
        
        // Barco
        spawn_visual_boat(300, &layout, &shared_city, Arc::clone(&agents));
        total += 1;
        
        Self {
            city: shared_city,
            layout,
            agents,
            cycle: 0,
            deliveries: 0,
            total_agents: total,
        }
    }
    
    pub fn step(&mut self) {
        self.cycle += 1;
        
        // Ejecutar un ciclo de simulación
        run_simulation(1);
        
        // Actualizar plantas
        let mut city = self.city.lock().unwrap();
        city.update(100);
    }
}

// Funciones auxiliares para crear agentes visuales

fn spawn_visual_car(id: u32, layout: &CityLayout, city: &SharedCity, agents: Arc<Mutex<Vec<VisualAgent>>>) {
    use rand::Rng;
    let mut rng = rand::rng();
    
    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);
    
    // Agregar a lista visual
    agents.lock().unwrap().push(VisualAgent {
        id,
        agent_type: VisualAgentType::Car,
        pos: origin,
        dest,
    });
    
    // Crear hilo (simplificado)
    let city_clone = Arc::clone(city);
    let agents_clone = Arc::clone(&agents);
    let mut pos = origin;
    
    my_thread_create(
        &format!("Car-{}", id),
        SchedulerParams::Lottery { tickets: 10 },
        Box::new(move |_| {
            // Actualizar posición visual
            if let Some(agent) = agents_clone.lock().unwrap().iter_mut().find(|a| a.id == id) {
                agent.pos = pos;
            }
            
            // Lógica simple de movimiento
            if pos.x == dest.x && pos.y == dest.y {
                // Remover de lista visual
                agents_clone.lock().unwrap().retain(|a| a.id != id);
                return ThreadSignal::Exit;
            }
            
            // Moverse
            if pos.y < dest.y {
                pos.y += 1;
            } else if pos.y > dest.y {
                pos.y -= 1;
            } else if pos.x < dest.x {
                pos.x += 1;
            } else if pos.x > dest.x {
                pos.x -= 1;
            }
            
            ThreadSignal::Yield
        }),
    );
}

fn spawn_visual_ambulance(id: u32, layout: &CityLayout, city: &SharedCity, agents: Arc<Mutex<Vec<VisualAgent>>>) {
    use rand::Rng;
    let mut rng = rand::rng();
    
    let origin = random_position(&mut rng, layout);
    let dest = random_destination(&mut rng, layout, origin);
    
    agents.lock().unwrap().push(VisualAgent {
        id,
        agent_type: VisualAgentType::Ambulance,
        pos: origin,
        dest,
    });
    
    let city_clone = Arc::clone(city);
    let agents_clone = Arc::clone(&agents);
    let mut pos = origin;
    
    my_thread_create(
        &format!("Ambulance-{}", id),
        SchedulerParams::Lottery { tickets: 100 },
        Box::new(move |_| {
            if let Some(agent) = agents_clone.lock().unwrap().iter_mut().find(|a| a.id == id) {
                agent.pos = pos;
            }
            
            if pos.x == dest.x && pos.y == dest.y {
                agents_clone.lock().unwrap().retain(|a| a.id != id);
                return ThreadSignal::Exit;
            }
            
            if pos.y < dest.y {
                pos.y += 1;
            } else if pos.y > dest.y {
                pos.y -= 1;
            } else if pos.x < dest.x {
                pos.x += 1;
            } else if pos.x > dest.x {
                pos.x -= 1;
            }
            
            ThreadSignal::Yield
        }),
    );
}

fn spawn_visual_truck(
    id: u32,
    cargo: SupplyKind,
    origin: (u32, u32),
    dest: (u32, u32),
    layout: &CityLayout,
    city: &SharedCity,
    agents: Arc<Mutex<Vec<VisualAgent>>>,
) {
    let origin_coord = Coord::new(origin.0, origin.1);
    let dest_coord = Coord::new(dest.0, dest.1);
    
    agents.lock().unwrap().push(VisualAgent {
        id,
        agent_type: VisualAgentType::Truck,
        pos: origin_coord,
        dest: dest_coord,
    });
    
    let deadline = {
        let city_lock = city.lock().unwrap();
        let plant = city_lock.plants.iter()
            .find(|p| p.loc.x == dest_coord.x && p.loc.y == dest_coord.y)
            .expect("Planta no encontrada");
        
        let supply = plant.requires.iter()
            .find(|s| s.kind == cargo)
            .expect("Suministro no requerido");
        
        city_lock.current_time() + supply.deadline_ms
    };
    
    let city_clone = Arc::clone(city);
    let agents_clone = Arc::clone(&agents);
    let mut pos = origin_coord;
    
    my_thread_create(
        &format!("Truck-{}", id),
        SchedulerParams::RealTime { deadline },
        Box::new(move |_| {
            if let Some(agent) = agents_clone.lock().unwrap().iter_mut().find(|a| a.id == id) {
                agent.pos = pos;
            }
            
            if pos.x == dest_coord.x && pos.y == dest_coord.y {
                // Entregar
                let mut city_lock = city_clone.lock().unwrap();
                let current_time = city_lock.current_time();
                
                if let Some(plant) = city_lock.find_plant_at(dest_coord) {
                    let supply = plant.requires.iter()
                        .find(|s| s.kind == cargo)
                        .expect("Suministro no requerido")
                        .clone();
                    
                    plant.commit_delivery(supply, current_time);
                }
                drop(city_lock);
                
                agents_clone.lock().unwrap().retain(|a| a.id != id);
                return ThreadSignal::Exit;
            }
            
            if pos.x < dest_coord.x {
                pos.x += 1;
            } else if pos.x > dest_coord.x {
                pos.x -= 1;
            } else if pos.y < dest_coord.y {
                pos.y += 1;
            } else if pos.y > dest_coord.y {
                pos.y -= 1;
            }
            
            ThreadSignal::Yield
        }),
    );
}

fn spawn_visual_boat(id: u32, layout: &CityLayout, city: &SharedCity, agents: Arc<Mutex<Vec<VisualAgent>>>) {
    let origin = Coord::new(layout.bridge1_row, layout.river_column);
    let dest = Coord::new(layout.bridge3_row + 1, layout.river_column);
    
    agents.lock().unwrap().push(VisualAgent {
        id,
        agent_type: VisualAgentType::Boat,
        pos: origin,
        dest,
    });
    
    let city_clone = Arc::clone(city);
    let agents_clone = Arc::clone(&agents);
    let mut pos = origin;
    
    my_thread_create(
        &format!("Boat-{}", id),
        SchedulerParams::RoundRobin,
        Box::new(move |_| {
            if let Some(agent) = agents_clone.lock().unwrap().iter_mut().find(|a| a.id == id) {
                agent.pos = pos;
            }
            
            if pos.x == dest.x && pos.y == dest.y {
                agents_clone.lock().unwrap().retain(|a| a.id != id);
                return ThreadSignal::Exit;
            }
            
            if pos.x < dest.x {
                pos.x += 1;
            }
            
            ThreadSignal::Yield
        }),
    );
}

fn random_position(rng: &mut rand::prelude::ThreadRng, layout: &CityLayout) -> Coord {
    use rand::Rng;
    let row = rng.random_range(0..layout.grid_rows);
    let col = if rng.random_bool(0.5) {
        rng.random_range(0..layout.river_column)
    } else {
        rng.random_range((layout.river_column + 1)..layout.grid_cols)
    };
    Coord::new(row, col)
}

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
