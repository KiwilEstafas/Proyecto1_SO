// Puentes con diferentes reglas de tráfico

use mypthreads::channels::SimpleMutex;
use mypthreads::thread::ThreadId;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    NorthToSouth,
    SouthToNorth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeType {
    TrafficLight,  // Puente 1: semáforo
    Yield,         // Puente 2: ceda el paso
    Drawbridge,    // Puente 3: levadizo para barcos
}

/// Estado interno del puente protegido por mutex
#[derive(Debug)]
struct BridgeState {
    vehicles_crossing: u32,
    current_direction: Option<TrafficDirection>,
    boat_passing: bool,
    light_state: TrafficLightState,
    last_light_change_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrafficLightState {
    NorthGreen,
    SouthGreen,
}

pub struct Bridge {
    pub id: u32,
    pub bridge_type: BridgeType,
    pub row: u32,
    pub capacity: u32,
    
    // Estado protegido
    state: Arc<Mutex<BridgeState>>,
    
    // Mutex para sincronización de hilos
    access_mutex: SimpleMutex,
    
    // Cola de espera con prioridades
    wait_queue: Arc<Mutex<VecDeque<(ThreadId, u8, TrafficDirection)>>>,
    
    // Configuración específica
    light_cycle_ms: u64,
    priority_direction: TrafficDirection,
}

impl Bridge {
    /// Crea un puente con semáforo (Puente 1)
    pub fn new_traffic_light(id: u32, row: u32, cycle_ms: u64) -> Self {
        Self {
            id,
            bridge_type: BridgeType::TrafficLight,
            row,
            capacity: 1,
            state: Arc::new(Mutex::new(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            })),
            access_mutex: SimpleMutex::new(),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            light_cycle_ms: cycle_ms,
            priority_direction: TrafficDirection::NorthToSouth,
        }
    }
    
    /// Crea un puente con ceda el paso (Puente 2)
    pub fn new_yield(id: u32, row: u32, priority_dir: TrafficDirection) -> Self {
        Self {
            id,
            bridge_type: BridgeType::Yield,
            row,
            capacity: 1,
            state: Arc::new(Mutex::new(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            })),
            access_mutex: SimpleMutex::new(),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            light_cycle_ms: 0,
            priority_direction: priority_dir,
        }
    }
    
    /// Crea un puente levadizo (Puente 3)
    pub fn new_drawbridge(id: u32, row: u32) -> Self {
        Self {
            id,
            bridge_type: BridgeType::Drawbridge,
            row,
            capacity: 2,
            state: Arc::new(Mutex::new(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            })),
            access_mutex: SimpleMutex::new(),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            light_cycle_ms: 0,
            priority_direction: TrafficDirection::NorthToSouth,
        }
    }
    
    /// Actualizar el estado del puente (para semáforos)
    pub fn update(&mut self, current_time_ms: u64) {
        if self.bridge_type != BridgeType::TrafficLight {
            return;
        }
        
        let mut state = self.state.lock().unwrap();
        
        if current_time_ms - state.last_light_change_ms >= self.light_cycle_ms {
            // Cambiar el semáforo
            state.light_state = match state.light_state {
                TrafficLightState::NorthGreen => TrafficLightState::SouthGreen,
                TrafficLightState::SouthGreen => TrafficLightState::NorthGreen,
            };
            state.last_light_change_ms = current_time_ms;
            
            println!("[Puente {}] Semáforo cambió a {:?}", self.id, state.light_state);
        }
    }
    
    /// Intentar cruzar el puente (vehículo)
    pub fn try_cross(&self, tid: ThreadId, priority: u8, direction: TrafficDirection) -> bool {
        let mut state = self.state.lock().unwrap();
        
        // No se puede cruzar si hay un barco
        if state.boat_passing {
            return false;
        }
        
        // Verificar según el tipo de puente
        let can_cross = match self.bridge_type {
            BridgeType::TrafficLight => {
                let light_allows = match state.light_state {
                    TrafficLightState::NorthGreen => direction == TrafficDirection::NorthToSouth,
                    TrafficLightState::SouthGreen => direction == TrafficDirection::SouthToNorth,
                };
                
                light_allows && state.vehicles_crossing < self.capacity
            }
            
            BridgeType::Yield => {
                if state.vehicles_crossing >= self.capacity {
                    return false;
                }
                
                // Si el puente está vacío, puede pasar
                if state.vehicles_crossing == 0 {
                    true
                } else {
                    // Solo puede pasar si va en la misma dirección
                    state.current_direction == Some(direction)
                }
            }
            
            BridgeType::Drawbridge => {
                state.vehicles_crossing < self.capacity &&
                (state.vehicles_crossing == 0 || state.current_direction == Some(direction))
            }
        };
        
        if can_cross {
            state.vehicles_crossing += 1;
            state.current_direction = Some(direction);
            println!("[Puente {}] Vehículo {} cruzando (dir: {:?}, total: {})", 
                     self.id, tid, direction, state.vehicles_crossing);
            true
        } else {
            false
        }
    }
    
    /// Salir del puente (vehículo)
    pub fn exit_bridge(&self, tid: ThreadId) {
        let mut state = self.state.lock().unwrap();
        
        if state.vehicles_crossing > 0 {
            state.vehicles_crossing -= 1;
            println!("[Puente {}] Vehículo {} salió (restantes: {})", 
                     self.id, tid, state.vehicles_crossing);
            
            if state.vehicles_crossing == 0 {
                state.current_direction = None;
            }
        }
    }
    
    /// Un barco solicita pasar (solo para Drawbridge)
    pub fn boat_request_pass(&self) -> bool {
        if self.bridge_type != BridgeType::Drawbridge {
            return false;
        }
        
        let mut state = self.state.lock().unwrap();
        
        if state.vehicles_crossing == 0 && !state.boat_passing {
            state.boat_passing = true;
            println!("[Puente {}] Barco comenzando a pasar, puente levadizo ARRIBA", self.id);
            true
        } else {
            false
        }
    }
    
    /// Un barco termina de pasar
    pub fn boat_exit(&self) {
        let mut state = self.state.lock().unwrap();
        state.boat_passing = false;
        println!("[Puente {}] Barco terminó de pasar, puente levadizo ABAJO", self.id);
    }
    
    /// Obtener el mutex de acceso
    pub fn get_mutex(&self) -> &SimpleMutex {
        &self.access_mutex
    }
}