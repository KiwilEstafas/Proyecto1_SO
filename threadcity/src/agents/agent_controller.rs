// Controlador de agentes que usa ThreadSignal

use crate::agents::{Agent, AgentDowncast, Car, Ambulance, Boat, CargoTruck};
use crate::model::{Bridge, Coord, TrafficDirection};
use mypthreads::signals::ThreadSignal;
use std::sync::{Arc, Mutex};

/// Estado de un agente durante la simulación
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPhase {
    Traveling,
    ApproachingBridge,
    WaitingForBridge,
    CrossingBridge(u32), // contador de progreso
    Arrived,
}

/// Contexto compartido para un agente
pub struct AgentContext {
    pub agent: Box<dyn AgentDowncast + Send>,
    pub phase: AgentPhase,
    pub destination: Coord,
    pub river_column: u32,
    pub bridges: Vec<Arc<Bridge>>,
}

impl AgentContext {
    pub fn new(
        agent: Box<dyn AgentDowncast + Send>,
        destination: Coord,
        river_column: u32,
        bridges: Vec<Arc<Bridge>>,
    ) -> Self {
        Self {
            agent,
            phase: AgentPhase::Traveling,
            destination,
            river_column,
            bridges,
        }
    }

    /// Función principal del agente que retorna ThreadSignal
    pub fn step(&mut self) -> ThreadSignal {
        let pos = self.agent.pos();
        let dest = self.destination;

        match self.phase {
            AgentPhase::Traveling => {
                // Verificar si llegó al destino
                if pos.x == dest.x && pos.y == dest.y {
                    println!("[Agent-{}] ✅ LLEGÓ al destino", self.agent.id());
                    self.phase = AgentPhase::Arrived;
                    return ThreadSignal::Exit;
                }

                // Verificar si necesita cruzar el río
                let needs_bridge = (pos.y < self.river_column && dest.y > self.river_column)
                    || (pos.y > self.river_column && dest.y < self.river_column);

                if needs_bridge {
                    let at_entrance = if dest.y > self.river_column {
                        pos.y == self.river_column - 1
                    } else {
                        pos.y == self.river_column + 1
                    };

                    if at_entrance {
                        println!(
                            "[Agent-{}] 🌉 Llegando a entrada del puente",
                            self.agent.id()
                        );
                        self.phase = AgentPhase::ApproachingBridge;
                        return ThreadSignal::Yield;
                    }
                }

                // Moverse hacia el destino
                self.move_towards_destination();
                ThreadSignal::Yield
            }

            AgentPhase::ApproachingBridge => {
                // Determinar qué puente usar
                let bridge = self.select_nearest_bridge();
                let direction = self.get_crossing_direction();

                // Barcos van directo al puente levadizo
                if self.is_boat() {
                    let signal = bridge.request_pass_boat();
                    if signal == ThreadSignal::Continue {
                        self.phase = AgentPhase::CrossingBridge(0);
                    } else {
                        self.phase = AgentPhase::WaitingForBridge;
                    }
                    return signal;
                }

                // Vehículos terrestres
                let is_ambulance = self.is_ambulance();
                let signal = bridge.request_pass_vehicle(direction, is_ambulance);

                if signal == ThreadSignal::Continue {
                    self.phase = AgentPhase::CrossingBridge(0);
                } else {
                    self.phase = AgentPhase::WaitingForBridge;
                }

                signal
            }

            AgentPhase::WaitingForBridge => {
                // Reintentar pedir paso
                let bridge = self.select_nearest_bridge();
                let direction = self.get_crossing_direction();

                if self.is_boat() {
                    let signal = bridge.request_pass_boat();
                    if signal == ThreadSignal::Continue {
                        self.phase = AgentPhase::CrossingBridge(0);
                    }
                    return signal;
                }

                let is_ambulance = self.is_ambulance();
                let signal = bridge.request_pass_vehicle(direction, is_ambulance);

                if signal == ThreadSignal::Continue {
                    self.phase = AgentPhase::CrossingBridge(0);
                }

                signal
            }

            AgentPhase::CrossingBridge(progress) => {
                let crossing_time = if self.is_boat() { 5 } else { 3 };

                if progress >= crossing_time {
                    // Terminó de cruzar
                    println!("[Agent-{}] ✅ Cruzó el puente", self.agent.id());

                    let bridge = self.select_nearest_bridge();
                    if self.is_boat() {
                        bridge.release_pass_boat();
                    } else {
                        bridge.release_pass_vehicle();
                    }

                    // Actualizar posición
                    let mut new_pos = self.agent.pos();
                    if self.is_boat() {
                        new_pos.x += 1; // Barco sigue por el río
                    } else {
                        // Vehículo cruza horizontalmente
                        if new_pos.y < self.river_column {
                            new_pos.y = self.river_column + 1;
                        } else {
                            new_pos.y = self.river_column - 1;
                        }
                    }
                    self.agent.set_pos(new_pos);

                    self.phase = AgentPhase::Traveling;
                } else {
                    self.phase = AgentPhase::CrossingBridge(progress + 1);
                }

                ThreadSignal::Yield
            }

            AgentPhase::Arrived => ThreadSignal::Exit,
        }
    }

    fn move_towards_destination(&mut self) {
        let pos = self.agent.pos();
        let dest = self.destination;
        let mut new_pos = pos;

        if pos.y < dest.y {
            new_pos.y += 1;
        } else if pos.y > dest.y {
            new_pos.y -= 1;
        } else if pos.x < dest.x {
            new_pos.x += 1;
        } else if pos.x > dest.x {
            new_pos.x -= 1;
        }

        self.agent.set_pos(new_pos);
    }

    fn get_crossing_direction(&self) -> TrafficDirection {
        let pos = self.agent.pos();
        if pos.y < self.river_column {
            TrafficDirection::WestToEast
        } else {
            TrafficDirection::EastToWest
        }
    }

    fn select_nearest_bridge(&self) -> &Arc<Bridge> {
        let pos = self.agent.pos();
        self.bridges
            .iter()
            .min_by_key(|b| (pos.x as i32 - b.row as i32).abs())
            .expect("No bridges available")
    }

    fn is_boat(&self) -> bool {
        self.agent.as_any().downcast_ref::<Boat>().is_some()
    }

    fn is_ambulance(&self) -> bool {
        self.agent.as_any().downcast_ref::<Ambulance>().is_some()
    }
}
