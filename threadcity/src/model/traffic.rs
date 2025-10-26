// estados de semaforo y ceda el paso

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficLightState {
    Red,
    Green,
}

#[derive(Debug, Default)]
pub struct YieldSign;

