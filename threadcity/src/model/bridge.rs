use mypthreads::sync::{shared, Shared};
use mypthreads::thread::ThreadId;
use std::collections::BinaryHeap;

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

// El struct que guardaremos en la cola. 
#[derive(Debug, Eq, PartialEq)]
struct WaitingVehicle {
    priority: u8,
    tid: ThreadId,
    direction: TrafficDirection,
}

// Implementación para que BinaryHeap sepa cómo ordenar (mayor prioridad primero)
impl Ord for WaitingVehicle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}
impl PartialOrd for WaitingVehicle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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
            // --- Y AHORA ESTO FUNCIONA ---
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

    /// Crea un puente con ceda el paso (Puente 2)
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
            wait_queue:shared(BinaryHeap::new()),
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
        if self.bridge_type != BridgeType::TrafficLight {
            return;
        }

        // Usar try_enter en lugar de lock() directo
        if let Some(mut state) = self.state.try_enter() {
            if current_time_ms - state.last_light_change_ms >= self.light_cycle_ms {
                // Cambiar el semáforo
                state.light_state = match state.light_state {
                    TrafficLightState::NorthGreen => TrafficLightState::SouthGreen,
                    TrafficLightState::SouthGreen => TrafficLightState::NorthGreen,
                };
                state.last_light_change_ms = current_time_ms;

                println!(
                    "[Puente {}] Semáforo cambió a {:?}",
                    self.id, state.light_state
                );
            }
            drop(state);
            let _ = self.state.request_unlock();
        }
        // Si no pudimos adquirir el lock, simplemente continuamos
        // (el semáforo se actualizará en el siguiente tick)
    }

    /// Intentar cruzar el puente (vehículo)
    pub fn try_cross(&self, tid: ThreadId, priority: u8, direction: TrafficDirection) -> bool {
        // Usar try_enter (no bloqueante)
        if let Some(mut state) = self.state.try_enter() {
            // No se puede cruzar si hay un barco (esta regla es absoluta)
            if state.boat_passing {
                drop(state);
                let _ = self.state.request_unlock();
                return false;
            }

            // --- REGLA DE PRIORIDAD ALTA ---
            if priority > 50 {
                if state.vehicles_crossing < self.capacity {
                    println!(
                        "[Puente {}] ¡ACCESO PRIORITARIO! Vehículo {} (prio:{}) cruzando.",
                        self.id, tid, priority
                    );
                    state.vehicles_crossing += 1;
                    if state.vehicles_crossing == 1 {
                        state.current_direction = Some(direction);
                    }
                    drop(state);
                    let _ = self.state.request_unlock();
                    return true;
                } else {
                    drop(state);
                    let _ = self.state.request_unlock();
                    return false;
                }
            }

            // --- LÓGICA NORMAL (para vehículos sin prioridad) ---
            let can_cross = match self.bridge_type {
                BridgeType::TrafficLight => {
                    let light_allows = match state.light_state {
                        TrafficLightState::NorthGreen => {
                            direction == TrafficDirection::NorthToSouth
                        }
                        TrafficLightState::SouthGreen => {
                            direction == TrafficDirection::SouthToNorth
                        }
                    };

                    light_allows && state.vehicles_crossing < self.capacity
                }

                BridgeType::Yield => {
                    if state.vehicles_crossing >= self.capacity {
                        false
                    } else if state.vehicles_crossing == 0 {
                        true
                    } else {
                        state.current_direction == Some(direction)
                    }
                }

                BridgeType::Drawbridge => {
                    state.vehicles_crossing < self.capacity
                        && (state.vehicles_crossing == 0
                            || state.current_direction == Some(direction))
                }
            };

            if can_cross {
                state.vehicles_crossing += 1;
                state.current_direction = Some(direction);
                println!(
                    "[Puente {}] Vehículo {} cruzando (dir: {:?}, total: {})",
                    self.id, tid, direction, state.vehicles_crossing
                );
                drop(state);
                let _ = self.state.request_unlock();
                true
            } else {
                drop(state);
                let _ = self.state.request_unlock();
                false
            }
        } else {
            // Si no podemos adquirir el lock, el hilo debe reintentar
            false
        }
    }

    /// Salir del puente (vehículo)
    pub fn exit_bridge(&self, tid: ThreadId) {
        // Usar try_enter
        if let Some(mut state) = self.state.try_enter() {
            if state.vehicles_crossing > 0 {
                state.vehicles_crossing -= 1;
                println!(
                    "[Puente {}] Vehículo {} salió (restantes: {})",
                    self.id, tid, state.vehicles_crossing
                );

                if state.vehicles_crossing == 0 {
                    state.current_direction = None;
                }
            }
            drop(state);
            let _ = self.state.request_unlock();
        }
    }

    /// Un barco solicita pasar (solo para Drawbridge)
    pub fn boat_request_pass(&self) -> bool {
        if self.bridge_type != BridgeType::Drawbridge {
            return false;
        }

        // Usar try_enter
        if let Some(mut state) = self.state.try_enter() {
            let ok = if state.vehicles_crossing == 0 && !state.boat_passing {
                state.boat_passing = true;
                println!(
                    "[Puente {}] Barco comenzando a pasar, puente levadizo ARRIBA",
                    self.id
                );
                true
            } else {
                false
            };
            drop(state);
            let _ = self.state.request_unlock();
            ok
        } else {
            false
        }
    }

    /// Un barco termina de pasar
    pub fn boat_exit(&self) {
        // Usar try_enter
        if let Some(mut state) = self.state.try_enter() {
            state.boat_passing = false;
            println!(
                "[Puente {}] Barco terminó de pasar, puente levadizo ABAJO",
                self.id
            );
            drop(state);
            let _ = self.state.request_unlock();
        }
    }
}
