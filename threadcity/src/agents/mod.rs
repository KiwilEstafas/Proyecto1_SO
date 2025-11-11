mod vehicle;
mod car;
mod ambulance;
mod boat;
mod cargotruck;

pub use vehicle::{Agent, AgentDowncast, Vehicle, AgentState, AgentType,AgentInfo};
pub use car::Car;
pub use ambulance::Ambulance;
pub use boat::Boat;
pub use cargotruck::CargoTruck;

