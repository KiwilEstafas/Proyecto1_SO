// planta nuclear y logistica de suministros
use crate::Coord;
use std::collections::HashMap;
use crate::tc_log; 


/// Tipo de insumo requerido por la planta
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupplyKind { Radioactive, Water }


/// Especificación de un insumo requerido
#[derive(Debug, Clone, Copy)]
pub struct SupplySpec {
    pub kind: SupplyKind,
    pub deadline_ms: u64, 
    pub period_ms: u64,
}


/// Política de deadlines para la planta
#[derive(Debug, Clone, Copy)]
pub struct DeadlinePolicy { pub max_lateness_ms: u64 }


/// Estado de la planta nuclear
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantStatus { Ok, AtRisk, Exploded }


/// Estructura que representa una planta nuclear
#[derive(Debug, Clone)]
pub struct NuclearPlant {
    pub id: u32,
    pub status: PlantStatus,
    pub loc: Coord,
    pub requires: Vec<SupplySpec>,
    pub deadline_policy: DeadlinePolicy,
    last_delivery_ms: HashMap<SupplyKind, u64>,
    risk_active: HashMap<SupplyKind, bool>,
    last_emergency_at_ms: Option<u64>,
    guard_fraction: f32,
}

impl NuclearPlant {
    pub fn new(id: u32, loc: Coord, requires: Vec<SupplySpec>, deadline_policy: DeadlinePolicy) -> Self {
        let mut risk_active = HashMap::new();
        risk_active.insert(SupplyKind::Radioactive, false);
        risk_active.insert(SupplyKind::Water, false);

        Self {
            id, status: PlantStatus::Ok, loc, requires, deadline_policy,
            last_delivery_ms: HashMap::new(),
            risk_active, last_emergency_at_ms: None, guard_fraction: 0.20,
        }
    }

    /// Registrar la entrega de un insumo
    pub fn commit_delivery(&mut self, spec: SupplySpec, at_ms: u64) {
        self.last_delivery_ms.insert(spec.kind, at_ms);
        if let Some(flag) = self.risk_active.get_mut(&spec.kind) {
            if *flag {
                *flag = false;
                tc_log!("✅ Planta {} ha sido reabastecida con {:?} y ya no está en riesgo.", self.id, spec.kind);
            }
        }
        if self.current_deficit(at_ms, false).is_none() { self.status = PlantStatus::Ok; }
    }


    /// Obtener el tiempo de la última entrega de un insumo
    pub fn get_last_delivery_time(&self, kind: &SupplyKind) -> u64 {
        *self.last_delivery_ms.get(kind).unwrap_or(&0)
    }

    /// Reiniciar la planta después de una explosión
    pub fn reset(&mut self, current_time: u64) {
        tc_log!("☢️  Planta {} reiniciándose después de la explosión en tiempo {}ms.", self.id, current_time);
        self.status = PlantStatus::Ok;
        self.last_delivery_ms.insert(SupplyKind::Radioactive, current_time);
        self.last_delivery_ms.insert(SupplyKind::Water, current_time);
        for v in self.risk_active.values_mut() { *v = false; }
        self.last_emergency_at_ms = None;
    }


    /// Verificar el déficit actual de insumos
    fn current_deficit(&self, now_ms: u64, guard: bool) -> Option<SupplyKind> {
        for spec in &self.requires {
            let last = self.get_last_delivery_time(&spec.kind);
            let due = last.saturating_add(spec.period_ms);
            let hard_deadline = due.saturating_add(self.deadline_policy.max_lateness_ms);

            /// Si ya pasó el deadline, hay déficit
            if now_ms >= due { return Some(spec.kind); }
            if guard {
                let guard_ms = ((self.deadline_policy.max_lateness_ms as f32) * self.guard_fraction) as u64;
                if now_ms + guard_ms >= hard_deadline { return Some(spec.kind); }
            }
        }
        None
    }

    /// Eleva emergencia SOLO en transición. Devuelve el insumo si se elevó nueva.
    pub fn maybe_raise_emergency(&mut self, now_ms: u64) -> Option<SupplyKind> {
        match self.current_deficit(now_ms, true) {
            None => { self.resolve_if_recovered(); None }
            Some(kind) => {
                let was = *self.risk_active.get(&kind).unwrap_or(&false);
                if !was {
                    self.risk_active.insert(kind, true);
                    if self.status != PlantStatus::Exploded { self.status = PlantStatus::AtRisk; }
                    self.last_emergency_at_ms = Some(now_ms);
                    tc_log!("🚨 EMERGENCIA: Planta {} necesita {:?} urgentemente.", self.id, kind);
                    Some(kind)
                } else { None }
            }
        }
    }


    /// Resolver emergencias si ya no hay déficit
    fn resolve_if_recovered(&mut self) {
        let mut any = false;
        for (_, v) in self.risk_active.iter_mut() {
            if *v { *v = false; any = true; }
        }
        if any && self.status == PlantStatus::AtRisk { tc_log!("🟢 EMERGENCIA RESUELTA en planta {}", self.id); }
        if self.status != PlantStatus::Exploded { self.status = PlantStatus::Ok; }
    }

    /// Avanzar el estado de la planta (verificar emergencias)
    pub fn tick_emergency(&mut self, now_ms: u64) -> Option<SupplyKind> { self.maybe_raise_emergency(now_ms) }

    /// Obtener el tipo de riesgo activo, si hay alguno
    pub fn active_risk_kind(&self, now_ms: u64) -> Option<SupplyKind> {
        if let Some((&k, _)) = self.risk_active.iter().find(|(_, v)| **v) { Some(k) }
        else { self.current_deficit(now_ms, false) }
    }
}
