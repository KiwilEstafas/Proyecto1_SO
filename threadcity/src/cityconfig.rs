// threadcity/src/city_config.rs
// Configuración estática de ThreadCity con todos sus elementos

use crate::sim::City;
use crate::model::{DeadlinePolicy, SupplySpec, SupplyKind};

/// Coordenadas clave de la ciudad
pub struct CityLayout {
    pub grid_rows: u32,
    pub grid_cols: u32,
    pub river_column: u32,
    pub bridge1_row: u32,
    pub bridge2_row: u32,
    pub bridge3_row: u32,
    pub north_zone_start: u32,
    pub south_zone_start: u32,
}

impl Default for CityLayout {
    fn default() -> Self {
        Self {
            grid_rows: 5,
            grid_cols: 5,
            river_column: 2,      // Río en columna 2 (divide la ciudad)
            bridge1_row: 1,       // Puente 1 en fila 1
            bridge2_row: 2,       // Puente 2 en fila 2
            bridge3_row: 3,       // Puente 3 en fila 3
            north_zone_start: 0,
            south_zone_start: 3,
        }
    }
}

/// Crea y configura la ciudad completa con todos sus elementos estáticos
pub fn create_threadcity() -> (City, CityLayout) {
    let layout = CityLayout::default();
    
    let mut city = City::new(layout.grid_rows, layout.grid_cols);
    
    // ===================================================================
    // RÍO
    // ===================================================================
    city.add_river();
    println!("🌊 Río agregado en columna {}", layout.river_column);
    
    // ===================================================================
    // PUENTES
    // ===================================================================
    
    // Puente 1: Semáforo, 1 carril, fila 1
    city.add_bridge(1, 1);
    println!("🌉 Puente 1 (Semáforo): 1 carril en fila {}", layout.bridge1_row);
    
    // Puente 2: Ceda, 1 carril, fila 2
    city.add_bridge(2, 1);
    println!("🌉 Puente 2 (Ceda): 1 carril en fila {}", layout.bridge2_row);
    
    // Puente 3: Levadizo, 2 carriles, fila 3
    city.add_bridge(3, 2);
    println!("🌉 Puente 3 (Levadizo): 2 carriles en fila {}", layout.bridge3_row);
    
    // ===================================================================
    // COMERCIOS - Distribuidos en el grid 5x5
    // ===================================================================
    
    // Zona Oeste (antes del río, columnas 0-1)
    city.add_commerce(1, (0, 0));
    city.add_commerce(2, (0, 1));
    city.add_commerce(3, (1, 0));
    city.add_commerce(4, (1, 1));
    city.add_commerce(5, (2, 0));
    city.add_commerce(6, (2, 1));
    city.add_commerce(7, (3, 0));
    city.add_commerce(8, (3, 1));
    city.add_commerce(9, (4, 0));
    city.add_commerce(10, (4, 1));
    
    // Zona Este (después del río, columnas 3-4)
    city.add_commerce(11, (0, 3));

    city.add_commerce(13, (1, 3));
    city.add_commerce(14, (1, 4));
    city.add_commerce(15, (2, 3));
    city.add_commerce(16, (2, 4));
    city.add_commerce(17, (3, 3));
    city.add_commerce(18, (3, 4));
    city.add_commerce(19, (4, 3));
    city.add_commerce(20, (4, 4));
    
    println!("🏪 {} comercios distribuidos en la ciudad", 20);
    
    // ===================================================================
    // PLANTAS NUCLEARES
    // ===================================================================
    
    // Planta Nuclear 1: Zona Oeste
    let plant1_supplies = vec![
        SupplySpec {
            kind: SupplyKind::Radioactive,
            deadline_ms: 5_000,
            period_ms: 10_000,
        },
        SupplySpec {
            kind: SupplyKind::Water,
            deadline_ms: 600,
            period_ms: 8_000,
        },
    ];
    let plant1_policy = DeadlinePolicy {
        max_lateness_ms: 100,
    };
    city.add_nuclear_plant(1, (0,1),plant1_supplies, plant1_policy);
    println!("☢️  Planta Nuclear 1: Zona Oeste (crítica - scheduling RT)");
    
    // Planta Nuclear 2: Zona Este
    let plant2_supplies = vec![
        SupplySpec {
            kind: SupplyKind::Radioactive,
            deadline_ms: 6_000,
            period_ms: 12_000,
        },
        SupplySpec {
            kind: SupplyKind::Water,
            deadline_ms: 4_000,
            period_ms: 9_000,
        },
    ];
    let plant2_policy = DeadlinePolicy {
        max_lateness_ms: 1_500,
    };
    city.add_nuclear_plant(2, (1,2),plant2_supplies, plant2_policy);
    println!("☢️  Planta Nuclear 2: Zona Este (crítica - scheduling RT)");
    
    println!("\n✅ ThreadCity configurada exitosamente");
    println!("   Grid: {}x{} = {} cuadras", 
             layout.grid_rows, layout.grid_cols, 
             layout.grid_rows * layout.grid_cols);
    println!("   Río: Columna {} (vertical)", layout.river_column);
    println!("   Puentes: 3 (filas {}, {}, {})", 
             layout.bridge1_row, layout.bridge2_row, layout.bridge3_row);
    println!("   Comercios: 20");
    println!("   Plantas Nucleares: 2\n");
    
    (city, layout)
}

/// Obtiene las coordenadas de un puente específico
pub fn get_bridge_coords(layout: &CityLayout, bridge_id: u32) -> (u32, u32) {
    let row = match bridge_id {
        1 => layout.bridge1_row,
        2 => layout.bridge2_row,
        3 => layout.bridge3_row,
        _ => layout.bridge1_row,
    };
    (row, layout.river_column)
}

/// Verifica si una coordenada está en zona oeste (antes del río)
pub fn is_west_zone(layout: &CityLayout, col: u32) -> bool {
    col < layout.river_column
}

/// Verifica si una coordenada está en zona este (después del río)
pub fn is_east_zone(layout: &CityLayout, col: u32) -> bool {
    col > layout.river_column
}

/// Obtiene el puente más cercano a una posición dada
pub fn get_nearest_bridge(layout: &CityLayout, from_row: u32) -> u32 {
    let dist1 = (from_row as i32 - layout.bridge1_row as i32).abs();
    let dist2 = (from_row as i32 - layout.bridge2_row as i32).abs();
    let dist3 = (from_row as i32 - layout.bridge3_row as i32).abs();
    
    if dist1 <= dist2 && dist1 <= dist3 {
        1
    } else if dist2 <= dist3 {
        2
    } else {
        3
    }
}