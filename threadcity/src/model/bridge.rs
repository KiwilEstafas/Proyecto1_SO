// Versión v2 compatible con mypthreads preemptivo

use mypthreads::mypthreads_api::MyMutex;
use mypthreads::signals::ThreadSignal;
use std::sync::{Arc, Mutex};

/// Dirección del tráfico para controlar los puentes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    WestToEast,
    EastToWest,
}

/// Estado compartido del puente (protegido por Mutex)
#[derive(Debug, Default, Clone)]
struct BridgeState {
    vehicles_on_bridge: u32,
    direction_on_bridge: Option<TrafficDirection>,
    is_boat_passing: bool,
    max_capacity: u32,
}

/// Tipos de puente
#[derive(Debug, Clone)]
pub enum BridgeType {
    TrafficLight {
        lanes: u32,
        green_duration_ms: u64,
        current_direction: TrafficDirection,
        time_in_state: u64,
    },
    Yield {
        lanes: u32,
        priority_direction: TrafficDirection,
    },
    Drawbridge {
        lanes: u32,
    },
}

#[derive(Clone)]
pub struct Bridge {
    pub id: u32,
    pub row: u32,
    pub bridge_type: BridgeType,
    mutex: Arc<MyMutex>,
    state: Arc<Mutex<BridgeState>>,
}

impl Bridge {
    pub fn new_traffic_light(id: u32, row: u32, green_duration_ms: u64) -> Self {
        let lanes = 1;
        Self {
            id,
            row,
            bridge_type: BridgeType::TrafficLight {
                lanes,
                green_duration_ms,
                current_direction: TrafficDirection::WestToEast,
                time_in_state: 0,
            },
            mutex: Arc::new(mypthreads::mypthreads_api::my_mutex_init()),
            state: Arc::new(Mutex::new(BridgeState {
                max_capacity: lanes,
                ..Default::default()
            })),
        }
    }

    pub fn new_yield(id: u32, row: u32, priority_direction: TrafficDirection) -> Self {
        let lanes = 1;
        Self {
            id,
            row,
            bridge_type: BridgeType::Yield {
                lanes,
                priority_direction,
            },
            mutex: Arc::new(mypthreads::mypthreads_api::my_mutex_init()),
            state: Arc::new(Mutex::new(BridgeState {
                max_capacity: lanes,
                ..Default::default()
            })),
        }
    }

    pub fn new_drawbridge(id: u32, row: u32) -> Self {
        let lanes = 2;
        Self {
            id,
            row,
            bridge_type: BridgeType::Drawbridge { lanes },
            mutex: Arc::new(mypthreads::mypthreads_api::my_mutex_init()),
            state: Arc::new(Mutex::new(BridgeState {
                max_capacity: lanes,
                ..Default::default()
            })),
        }
    }

    pub fn step(&mut self, dt_ms: u64) {
        if let BridgeType::TrafficLight {
            ref mut time_in_state,
            ref mut current_direction,
            green_duration_ms,
            ..
        } = self.bridge_type
        {
            *time_in_state += dt_ms;
            if *time_in_state >= green_duration_ms {
                *time_in_state = 0;
                *current_direction = match *current_direction {
                    TrafficDirection::WestToEast => TrafficDirection::EastToWest,
                    TrafficDirection::EastToWest => TrafficDirection::WestToEast,
                };
                println!("[Puente {}] 🚦 Semáforo cambió a {:?}", self.id, *current_direction);
            }
        }
    }

    pub fn request_pass_vehicle(
        &self,
        direction: TrafficDirection,
        is_ambulance: bool,
    ) -> ThreadSignal {
        if is_ambulance {
            println!("[Puente {}] 🚑 Ambulancia pasa sin esperar", self.id);
            return ThreadSignal::Continue;
        }

        let lock_signal = mypthreads::mypthreads_api::my_mutex_lock(&self.mutex);
        if lock_signal != ThreadSignal::Continue {
            return lock_signal;
        }

        let mut state = self.state.lock().unwrap();

        let can_pass = match &self.bridge_type {
            BridgeType::TrafficLight {
                current_direction, ..
            } => {
                let light_is_green = *current_direction == direction;
                let has_space = state.vehicles_on_bridge < state.max_capacity;
                let same_dir = state.direction_on_bridge == Some(direction)
                    || state.direction_on_bridge.is_none();

                !state.is_boat_passing && has_space && light_is_green && same_dir
            }

            BridgeType::Yield {
                priority_direction, ..
            } => {
                let has_space = state.vehicles_on_bridge < state.max_capacity;
                let same_dir = state.direction_on_bridge == Some(direction)
                    || state.direction_on_bridge.is_none();

                if direction == *priority_direction {
                    !state.is_boat_passing && has_space && same_dir
                } else {
                    !state.is_boat_passing && state.vehicles_on_bridge == 0
                }
            }

            BridgeType::Drawbridge { .. } => {
                let has_space = state.vehicles_on_bridge < state.max_capacity;
                let same_dir = state.direction_on_bridge == Some(direction)
                    || state.direction_on_bridge.is_none();

                !state.is_boat_passing && has_space && same_dir
            }
        };

        if can_pass {
            state.vehicles_on_bridge += 1;
            state.direction_on_bridge = Some(direction);
            println!(
                "[Puente {}] 🚗 Vehículo entrando ({:?}). Ocupación: {}/{}",
                self.id, direction, state.vehicles_on_bridge, state.max_capacity
            );
            drop(state);
            mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
            return ThreadSignal::Continue;
        }

        drop(state);
        mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
        println!("[Puente {}] 🚫 Vehículo bloqueado ({:?})", self.id, direction);
        ThreadSignal::Block
    }

    pub fn release_pass_vehicle(&self) {
        mypthreads::mypthreads_api::my_mutex_lock(&self.mutex);
        let mut state = self.state.lock().unwrap();

        if state.vehicles_on_bridge > 0 {
            state.vehicles_on_bridge -= 1;
        }

        if state.vehicles_on_bridge == 0 {
            state.direction_on_bridge = None;
        }

        println!(
            "[Puente {}] ✅ Vehículo salió. Ocupación: {}/{}",
            self.id, state.vehicles_on_bridge, state.max_capacity
        );

        drop(state);
        mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
    }

    pub fn request_pass_boat(&self) -> ThreadSignal {
        let lock_signal = mypthreads::mypthreads_api::my_mutex_lock(&self.mutex);
        if lock_signal != ThreadSignal::Continue {
            return lock_signal;
        }

        let mut state = self.state.lock().unwrap();

        if state.vehicles_on_bridge > 0 || state.is_boat_passing {
            drop(state);
            mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
            println!("[Puente {}] ⛵ Barco bloqueado", self.id);
            return ThreadSignal::Block;
        }

        state.is_boat_passing = true;
        println!("[Puente {}] ⛵ Barco pasando, puente levantado", self.id);

        drop(state);
        mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
        ThreadSignal::Continue
    }

    pub fn release_pass_boat(&self) {
        mypthreads::mypthreads_api::my_mutex_lock(&self.mutex);
        let mut state = self.state.lock().unwrap();
        state.is_boat_passing = false;
        println!("[Puente {}] ✅ Barco salió, puente bajado", self.id);
        drop(state);
        mypthreads::mypthreads_api::my_mutex_unlock(&self.mutex);
    }
}
