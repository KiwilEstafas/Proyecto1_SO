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
    /// Lateness máximo permitido después del período.
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

    // Última entrega por insumo
    last_delivery_ms: HashMap<SupplyKind, u64>,
    
    risk_active: HashMap<SupplyKind, bool>,
    /// último timestamp de emergencia (informativo)
    last_emergency_at_ms: Option<u64>,
    /// margen “suave” para anticipar el hard deadline
    guard_fraction: f32, // 0.20 = 20%
}

impl NuclearPlant {
    pub fn new(
        id: u32,
        loc: Coord,
        requires: Vec<SupplySpec>,
        deadline_policy: DeadlinePolicy,
    ) -> Self {
        let mut risk_active = HashMap::new();
        risk_active.insert(SupplyKind::Radioactive, false);
        risk_active.insert(SupplyKind::Water, false);

        Self {
            id,
            status: PlantStatus::Ok,
            loc,
            requires,
            deadline_policy,
            last_delivery_ms: HashMap::new(),
            risk_active,
            last_emergency_at_ms: None,
            guard_fraction: 0.20,
        }
    }

    /// Aplica una entrega y apaga la emergencia del insumo si estaba activa.
    pub fn commit_delivery(&mut self, spec: SupplySpec, at_ms: u64) {
        self.last_delivery_ms.insert(spec.kind, at_ms);

        if let Some(flag) = self.risk_active.get_mut(&spec.kind) {
            if *flag {
                *flag = false;
                println!(
                    "✅ Planta {} ha sido reabastecida con {:?} y ya no está en riesgo.",
                    self.id, spec.kind
                );
            }
        }

        // Si no hay otros déficits, volvemos a Ok
        if self.current_deficit(at_ms, false).is_none() {
            self.status = PlantStatus::Ok;
        }
    }

    pub fn get_last_delivery_time(&self, kind: &SupplyKind) -> u64 {
        *self.last_delivery_ms.get(kind).unwrap_or(&0)
    }

    /// Reinicia la planta tras explosión. Limpia flags de riesgo y reinicia contadores.
    pub fn reset(&mut self, current_time: u64) {
        println!(
            "☢️  Planta {} reiniciándose después de la explosión en tiempo {}ms.",
            self.id, current_time
        );
        self.status = PlantStatus::Ok;
        self.last_delivery_ms
            .insert(SupplyKind::Radioactive, current_time);
        self.last_delivery_ms
            .insert(SupplyKind::Water, current_time);

        for v in self.risk_active.values_mut() {
            *v = false;
        }
        self.last_emergency_at_ms = None;
    }

    fn current_deficit(&self, now_ms: u64, guard: bool) -> Option<SupplyKind> {
        for spec in &self.requires {
            let last = self.get_last_delivery_time(&spec.kind);
            let due = last.saturating_add(spec.period_ms);
            let hard_deadline = due.saturating_add(self.deadline_policy.max_lateness_ms);

            if now_ms >= due {
                return Some(spec.kind);
            }
            if guard {
                let guard_ms =
                    ((self.deadline_policy.max_lateness_ms as f32) * self.guard_fraction) as u64;
                if now_ms + guard_ms >= hard_deadline {
                    return Some(spec.kind);
                }
            }
        }
        None
    }

    /// Eleva emergencia SOLO en transición. Devuelve el insumo si se elevó nueva.
    pub fn maybe_raise_emergency(&mut self, now_ms: u64) -> Option<SupplyKind> {
        match self.current_deficit(now_ms, true) {
            None => {
                self.resolve_if_recovered();
                None
            }
            Some(kind) => {
                let was = *self.risk_active.get(&kind).unwrap_or(&false);
                if !was {
                    self.risk_active.insert(kind, true);
                    if self.status != PlantStatus::Exploded {
                        self.status = PlantStatus::AtRisk;
                    }
                    self.last_emergency_at_ms = Some(now_ms);
                    println!(
                        "🚨 EMERGENCIA: Planta {} necesita {:?} urgentemente.",
                        self.id, kind
                    );
                    Some(kind)
                } else {
                    None
                }
            }
        }
    }

    fn resolve_if_recovered(&mut self) {
        let mut any = false;
        for (_, v) in self.risk_active.iter_mut() {
            if *v {
                *v = false;
                any = true;
            }
        }
        if any && self.status == PlantStatus::AtRisk {
            println!("🟢 EMERGENCIA RESUELTA en planta {}", self.id);
        }
        if self.status != PlantStatus::Exploded {
            self.status = PlantStatus::Ok;
        }
    }

    /// Útil para el loop externo: llama en cada tick.
    pub fn tick_emergency(&mut self, now_ms: u64) -> Option<SupplyKind> {
        self.maybe_raise_emergency(now_ms)
    }

    pub fn active_risk_kind(&self, now_ms: u64) -> Option<SupplyKind> {
        // Si hay algún flag activo, úsalo; si no, consulta el déficit actual sin guard.
        if let Some((&k, _)) = self.risk_active.iter().find(|(_, v)| **v) {
            Some(k)
        } else {
            self.current_deficit(now_ms, false)
        }
    }
}
