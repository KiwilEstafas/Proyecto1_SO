// planta nuclear y logistica de suministros

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupplyKind { Radioactive, Water }

#[derive(Debug, Clone, Copy)]
pub struct SupplySpec {
    pub kind: SupplyKind,
    pub deadline_ms: u64,
    pub period_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DeadlinePolicy {
    pub max_lateness_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantStatus { Ok, AtRisk, Exploded }

#[derive(Debug)]
pub struct NuclearPlant {
    pub id: u32,
    pub status: PlantStatus,
    pub requires: Vec<SupplySpec>,
    pub deadline_policy: DeadlinePolicy,
    last_delivery_ms: u64,
}

impl NuclearPlant {
    pub fn new(id: u32, requires: Vec<SupplySpec>, deadline_policy: DeadlinePolicy) -> Self {
        Self {
            id,
            status: PlantStatus::Ok,
            requires,
            deadline_policy,
            last_delivery_ms: 0,
        }
    }

    pub fn commit_delivery(&mut self, spec: SupplySpec, at_ms: u64) {
        self.last_delivery_ms = at_ms;
        // regla simple de demo: si llego tarde mas que la tolerancia se pone en riesgo
        if at_ms > spec.deadline_ms + self.deadline_policy.max_lateness_ms {
            self.status = PlantStatus::AtRisk;
        } else {
            self.status = PlantStatus::Ok;
        }
    }
}

