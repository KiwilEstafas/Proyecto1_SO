// planta nuclear y logistica de suministros
use crate::Coord;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupplyKind {
    Radioactive,
    Water,
}

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
pub enum PlantStatus {
    Ok,
    AtRisk,
    Exploded,
}

#[derive(Debug, Clone)]
pub struct NuclearPlant {
    pub id: u32,
    pub status: PlantStatus,
    pub loc: Coord,
    pub requires: Vec<SupplySpec>,
    pub deadline_policy: DeadlinePolicy,
    last_delivery_ms: HashMap<SupplyKind, u64>,
}

impl NuclearPlant {
    pub fn new(
        id: u32,
        loc: Coord,
        requires: Vec<SupplySpec>,
        deadline_policy: DeadlinePolicy,
    ) -> Self {
        Self {
            id,
            status: PlantStatus::Ok,
            loc,
            requires,
            deadline_policy,
            last_delivery_ms: HashMap::new(),
        }
    }

    pub fn commit_delivery(&mut self, spec: SupplySpec, at_ms: u64) {
        // Al recibir una entrega, el estado SIEMPRE vuelve a ser 'Ok'.
        if self.status == PlantStatus::AtRisk {
            println!(
                "✅ Planta {} ha sido reabastecida y ya no está en riesgo.",
                self.id
            );
        }
        self.last_delivery_ms.insert(spec.kind, at_ms);
        self.status = PlantStatus::Ok;
    }

    pub fn get_last_delivery_time(&self, kind: &SupplyKind) -> u64 {
        *self.last_delivery_ms.get(kind).unwrap_or(&0)
    }

    pub fn reset(&mut self, current_time: u64) {
        println!(
            "☢️  Planta {} reiniciándose después de la explosión en tiempo {}ms.",
            self.id, current_time
        );
        self.status = PlantStatus::Ok;
        // Reinicia los contadores de entrega usando el tiempo actual como punto de partida.
        // Esto le da a la planta un ciclo completo antes de volver a estar en riesgo.
        self.last_delivery_ms
            .insert(SupplyKind::Radioactive, current_time);
        self.last_delivery_ms
            .insert(SupplyKind::Water, current_time);
    }
}
