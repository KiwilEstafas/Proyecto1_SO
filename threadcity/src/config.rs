use crate::model::*;
use crate::sim::City;

/// Diseño de la ciudad
#[derive(Debug, Clone)]
pub struct CityLayout {
    pub grid_rows: u32,
    pub grid_cols: u32,
    pub river_column: u32,
    pub bridge1_row: u32,
    pub bridge2_row: u32,
    pub bridge3_row: u32,
}

impl Default for CityLayout {
    fn default() -> Self {
        Self {
            grid_rows: 5,
            grid_cols: 5,
            river_column: 2,
            bridge1_row: 1,
            bridge2_row: 2,
            bridge3_row: 3,
        }
    }
}

/// Crea una ciudad configurada según los requerimientos
pub fn create_city() -> (City, CityLayout) {
    let layout = CityLayout::default();
    let mut city = City::new(layout.grid_rows, layout.grid_cols);
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              Creando ThreadCity                           ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    // Agregar río
    city.add_river();
    println!("🌊 Río agregado en columna {}", layout.river_column);
    
    // Agregar puentes
    city.add_bridge(Bridge::new_traffic_light(1, layout.bridge1_row, 5000));
    println!("🌉 Puente 1 (Semáforo): fila {}, ciclo 5000ms", layout.bridge1_row);
    
    city.add_bridge(Bridge::new_yield(2, layout.bridge2_row, TrafficDirection::NorthToSouth));
    println!("🌉 Puente 2 (Ceda): fila {}, prioridad Norte->Sur", layout.bridge2_row);
    
    city.add_bridge(Bridge::new_drawbridge(3, layout.bridge3_row));
    println!("🌉 Puente 3 (Levadizo): fila {}, 2 carriles", layout.bridge3_row);
    
    // Agregar comercios (mínimo 25)
    let mut commerce_id = 1;
    for row in 0..layout.grid_rows {
        for col in 0..layout.grid_cols {
            // Saltar la columna del río
            if col == layout.river_column {
                continue;
            }
            city.add_commerce(commerce_id, (row, col));
            commerce_id += 1;
        }
    }
    println!("🏪 {} comercios distribuidos", commerce_id - 1);
    
    // Agregar plantas nucleares
    // Planta 1: Zona Oeste (antes del río)
    let plant1_supplies = vec![
        SupplySpec {
            kind: SupplyKind::Radioactive,
            deadline_ms: 15_000,
            period_ms: 30_000,
        },
        SupplySpec {
            kind: SupplyKind::Water,
            deadline_ms: 12_000,
            period_ms: 24_000,
        },
    ];
    let plant1_policy = DeadlinePolicy {
        max_lateness_ms: 3_000,
    };
    city.add_nuclear_plant(1, (1, 0), plant1_supplies, plant1_policy);
    println!("☢️  Planta Nuclear 1: (1, 0) - Zona Oeste");
    
    // Planta 2: Zona Este (después del río)
    let plant2_supplies = vec![
        SupplySpec {
            kind: SupplyKind::Radioactive,
            deadline_ms: 18_000,
            period_ms: 36_000,
        },
        SupplySpec {
            kind: SupplyKind::Water,
            deadline_ms: 15_000,
            period_ms: 30_000,
        },
    ];
    let plant2_policy = DeadlinePolicy {
        max_lateness_ms: 3_500,
    };
    city.add_nuclear_plant(2, (2, 4), plant2_supplies, plant2_policy);
    println!("☢️  Planta Nuclear 2: (2, 4) - Zona Este");
    
    println!("\n✅ ThreadCity configurada:");
    println!("   Grid: {}x{} = {} cuadras", 
             layout.grid_rows, layout.grid_cols, 
             layout.grid_rows * layout.grid_cols);
    println!("   Comercios: {}", commerce_id - 1);
    println!("   Plantas: 2");
    println!("   Puentes: 3\n");
    
    (city, layout)
}

/// Verifica si una posición está en la zona oeste (antes del río)
pub fn is_west_zone(layout: &CityLayout, col: u32) -> bool {
    col < layout.river_column
}

/// Verifica si una posición está en la zona este (después del río)
pub fn is_east_zone(layout: &CityLayout, col: u32) -> bool {
    col > layout.river_column
}

/// Encuentra el puente más cercano a una fila dada
pub fn nearest_bridge(layout: &CityLayout, from_row: u32) -> u32 {
    let bridges = [layout.bridge1_row, layout.bridge2_row, layout.bridge3_row];
    let mut nearest = 1;
    let mut min_dist = u32::MAX;
    
    for (idx, &bridge_row) in bridges.iter().enumerate() {
        let dist = (from_row as i32 - bridge_row as i32).abs() as u32;
        if dist < min_dist {
            min_dist = dist;
            nearest = (idx + 1) as u32;
        }
    }
    
    nearest
}