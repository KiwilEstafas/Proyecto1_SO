// Aca la idea es definir que info se ocupa para dibujar la simulacion 

use threadcity::{PlantStatus, SupplyKind};

// Un enum para saber qué tipo de agente dibujar
#[derive(Debug, Clone, Copy)]
pub enum AgentKind {
    Car,
    Ambulance,
    CargoTruck,
    Boat,
}

// Información mínima necesaria para dibujar un agente
#[derive(Debug, Clone)]
pub struct RenderableAgent {
    pub id: u32,
    pub kind: AgentKind,
    pub pos: (u32, u32),
}

// Información mínima para dibujar una planta nuclear
#[derive(Debug, Clone)]
pub struct RenderablePlant {
    pub id: u32,
    pub pos: (u32, u32),
    pub status: PlantStatus,
}

// El "snapshot" completo que la simulación enviará a la UI en cada tick
#[derive(Debug, Clone)]
pub struct SimulationState {
    pub tick: u32,
    pub time_ms: u64,
    pub agents: Vec<RenderableAgent>,
    pub plants: Vec<RenderablePlant>,
    // Se puede añadir más cosas, pero por el momento solo es el esqueleto 
}