use mypthreads::sync::{shared, Shared};
use mypthreads::thread::ThreadId;
use std::collections::BinaryHeap;
use crate::tc_log; 

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    NorthToSouth,
    SouthToNorth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeType {
    TrafficLight, // Puente 1: semáforo
    Yield,        // Puente 2: ceda el paso
    Drawbridge,   // Puente 3: levadizo para barcos
}

#[derive(Debug)]
struct BridgeState {
    vehicles_crossing: u32,
    current_direction: Option<TrafficDirection>,
    boat_passing: bool,
    light_state: TrafficLightState,
    last_light_change_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrafficLightState { NorthGreen, SouthGreen }

// El struct que guardaremos en la cola. 
#[derive(Debug, Eq, PartialEq)]
struct WaitingVehicle {
    priority: u8,
    tid: ThreadId,
    direction: TrafficDirection,
}

impl Ord for WaitingVehicle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.priority.cmp(&other.priority) }
}
impl PartialOrd for WaitingVehicle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

pub struct Bridge {
    pub id: u32,
    pub bridge_type: BridgeType,
    pub row: u32,
    pub capacity: u32,

    // Estado protegido 
    state: Shared<BridgeState>,

    // Cola de espera con prioridades 
    wait_queue: Shared<BinaryHeap<WaitingVehicle>>,
    light_cycle_ms: u64,
    priority_direction: TrafficDirection,
}

impl Bridge {
    pub fn new_traffic_light(id: u32, row: u32, cycle_ms: u64) -> Self {
        Self {
            id,
            bridge_type: BridgeType::TrafficLight,
            row,
            capacity: 1,
            state: shared(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            }),
            wait_queue: shared(BinaryHeap::new()),
            light_cycle_ms: cycle_ms,
            priority_direction: TrafficDirection::NorthToSouth,
        }
    }

    pub fn new_yield(id: u32, row: u32, priority_dir: TrafficDirection) -> Self {
        Self {
            id,
            bridge_type: BridgeType::Yield,
            row,
            capacity: 1,
            state: shared(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            }),
            wait_queue: shared(BinaryHeap::new()),
            light_cycle_ms: 0,
            priority_direction: priority_dir,
        }
    }

    pub fn new_drawbridge(id: u32, row: u32) -> Self {
        Self {
            id,
            bridge_type: BridgeType::Drawbridge,
            row,
            capacity: 2,
            state: shared(BridgeState {
                vehicles_crossing: 0,
                current_direction: None,
                boat_passing: false,
                light_state: TrafficLightState::NorthGreen,
                last_light_change_ms: 0,
            }),
            wait_queue: shared(BinaryHeap::new()),
            light_cycle_ms: 0,
            priority_direction: TrafficDirection::NorthToSouth,
        }
    }

    /// Actualizar el estado del puente (para semáforos)
    pub fn update(&mut self, current_time_ms: u64) {
        if self.bridge_type != BridgeType::TrafficLight { return; }

        if let Some(mut state) = self.state.try_enter() {
            if current_time_ms - state.last_light_change_ms >= self.light_cycle_ms {
                state.light_state = match state.light_state {
                    TrafficLightState::NorthGreen => TrafficLightState::SouthGreen,
                    TrafficLightState::SouthGreen => TrafficLightState::NorthGreen,
                };
                state.last_light_change_ms = current_time_ms;

                tc_log!("[Puente {}] Semáforo cambió a {:?}", self.id, state.light_state);
            }
            drop(state);
            let _ = self.state.request_unlock();
        }
    }

    /// Intentar cruzar el puente (vehículo)
    pub fn try_cross(&self, tid: ThreadId, priority: u8, direction: TrafficDirection) -> bool {
        if let Some(mut state) = self.state.try_enter() {
            if state.boat_passing {
                drop(state);
                let _ = self.state.request_unlock();
                return false;
            }

            if priority > 50 {
                if state.vehicles_crossing < self.capacity {
                    tc_log!("[Puente {}] ¡ACCESO PRIORITARIO! Vehículo {} (prio:{}) cruzando.", self.id, tid, priority);
                    state.vehicles_crossing += 1;
                    if state.vehicles_crossing == 1 { state.current_direction = Some(direction); }
                    drop(state);
                    let _ = self.state.request_unlock();
                    return true;
                } else {
                    drop(state);
                    let _ = self.state.request_unlock();
                    return false;
                }
            }

            let can_cross = match self.bridge_type {
                BridgeType::TrafficLight => {
                    let light_allows = match state.light_state {
                        TrafficLightState::NorthGreen => direction == TrafficDirection::NorthToSouth,
                        TrafficLightState::SouthGreen => direction == TrafficDirection::SouthToNorth,
                    };
                    light_allows && state.vehicles_crossing < self.capacity
                }
                BridgeType::Yield => {
                    if state.vehicles_crossing >= self.capacity { false }
                    else if state.vehicles_crossing == 0 { true }
                    else { state.current_direction == Some(direction) }
                }
                BridgeType::Drawbridge => {
                    state.vehicles_crossing < self.capacity &&
                    (state.vehicles_crossing == 0 || state.current_direction == Some(direction))
                }
            };

            if can_cross {
                state.vehicles_crossing += 1;
                state.current_direction = Some(direction);
                tc_log!("[Puente {}] Vehículo {} cruzando (dir: {:?}, total: {})", self.id, tid, direction, state.vehicles_crossing);
                drop(state);
                let _ = self.state.request_unlock();
                true
            } else {
                drop(state);
                let _ = self.state.request_unlock();
                false
            }
        } else {
            false
        }
    }

    pub fn exit_bridge(&self, tid: ThreadId) {
        if let Some(mut state) = self.state.try_enter() {
            if state.vehicles_crossing > 0 {
                state.vehicles_crossing -= 1;
                tc_log!("[Puente {}] Vehículo {} salió (restantes: {})", self.id, tid, state.vehicles_crossing);
                if state.vehicles_crossing == 0 { state.current_direction = None; }
            }
            drop(state);
            let _ = self.state.request_unlock();
        }
    }

    pub fn boat_request_pass(&self) -> bool {
        if self.bridge_type != BridgeType::Drawbridge { return false; }
        if let Some(mut state) = self.state.try_enter() {
            let ok = if state.vehicles_crossing == 0 && !state.boat_passing {
                state.boat_passing = true;
                tc_log!("[Puente {}] Barco comenzando a pasar, puente levadizo ARRIBA", self.id);
                true
            } else { false };
            drop(state);
            let _ = self.state.request_unlock();
            ok
        } else { false }
    }

    pub fn boat_exit(&self) {
        if let Some(mut state) = self.state.try_enter() {
            state.boat_passing = false;
            tc_log!("[Puente {}] Barco terminó de pasar, puente levadizo ABAJO", self.id);
            drop(state);
            let _ = self.state.request_unlock();
        }
    }
}
