// // threadcity/src/city_config.rs
// // Configuración corregida de ThreadCity

// use crate::sim::City;
// use crate::model::{DeadlinePolicy, SupplySpec, SupplyKind, Bridge, TrafficDirection};

// pub struct CityLayout {
//     pub grid_rows: u32,
//     pub grid_cols: u32,
//     pub river_column: u32,
//     pub bridge1_row: u32,
//     pub bridge2_row: u32,
//     pub bridge3_row: u32,
//     pub north_zone_start: u32,
//     pub south_zone_start: u32,
// }

// impl Default for CityLayout {
//     fn default() -> Self {
//         Self {
//             grid_rows: 5,
//             grid_cols: 5,
//             river_column: 2,
//             bridge1_row: 1,
//             bridge2_row: 2,
//             bridge3_row: 3,
//             north_zone_start: 0,
//             south_zone_start: 3,
//         }
//     }
// }

// pub fn create_threadcity() -> (City, CityLayout) {
//     let layout = CityLayout::default();

//     let mut city = City::new(layout.grid_rows, layout.grid_cols);

//     // RÍO
//     city.add_river();
//     println!("🌊 Río agregado en columna {}", layout.river_column);

//     // PUENTES
//     city.bridges.push(Bridge::new_traffic_light(1, 1, 5000));
//     println!("🌉 Puente 1 (Semáforo): 1 carril en fila {}, ciclo 5000ms", layout.bridge1_row);

//     city.bridges.push(Bridge::new_yield(2, 1, TrafficDirection::WestToEast));
//     println!("🌉 Puente 2 (Ceda): 1 carril en fila {}, prioridad Oeste->Este", layout.bridge2_row);

//     city.bridges.push(Bridge::new_drawbridge(3, 2));
//     println!("🌉 Puente 3 (Levadizo): 2 carriles en fila {}", layout.bridge3_row);

//     // COMERCIOS
//     city.add_commerce(1, (0, 0));
//     city.add_commerce(2, (0, 1));
//     city.add_commerce(3, (1, 0));
//     city.add_commerce(4, (1, 1));
//     city.add_commerce(5, (2, 0));
//     city.add_commerce(6, (2, 1));
//     city.add_commerce(7, (3, 0));
//     city.add_commerce(8, (3, 1));
//     city.add_commerce(9, (4, 0));
//     city.add_commerce(10, (4, 1));
//     city.add_commerce(11, (0, 3));
//     city.add_commerce(12, (0, 4));
//     city.add_commerce(13, (1, 3));
//     city.add_commerce(14, (1, 4));
//     city.add_commerce(15, (2, 3));
//     city.add_commerce(16, (2, 4));
//     city.add_commerce(17, (3, 3));
//     city.add_commerce(18, (3, 4));
//     city.add_commerce(19, (4, 3));
//     city.add_commerce(20, (4, 4));

//     println!("🏪 {} comercios distribuidos en la ciudad", 20);

//     // PLANTAS NUCLEARES - CORREGIDAS
//     // Planta 1: Zona OESTE (columna 1, antes del río que está en columna 2)
//     let plant1_supplies = vec![
//         SupplySpec {
//             kind: SupplyKind::Radioactive,
//             deadline_ms: 10_000,  // Aumentado para dar más tiempo
//             period_ms: 20_000,
//         },
//         SupplySpec {
//             kind: SupplyKind::Water,
//             deadline_ms: 8_000,   // Aumentado para dar más tiempo
//             period_ms: 16_000,
//         },
//     ];
//     let plant1_policy = DeadlinePolicy {
//         max_lateness_ms: 2_000,  // Aumentado
//     };
//     city.add_nuclear_plant(1, (0, 1), plant1_supplies, plant1_policy);
//     println!("☢️  Planta Nuclear 1: Zona Oeste (0, 1) - columna 1");

//     // Planta 2: Zona ESTE (columna 3, después del río que está en columna 2)
//     let plant2_supplies = vec![
//         SupplySpec {
//             kind: SupplyKind::Radioactive,
//             deadline_ms: 12_000,  // Aumentado
//             period_ms: 24_000,
//         },
//         SupplySpec {
//             kind: SupplyKind::Water,
//             deadline_ms: 10_000,  // Aumentado
//             period_ms: 20_000,
//         },
//     ];
//     let plant2_policy = DeadlinePolicy {
//         max_lateness_ms: 2_500,  // Aumentado
//     };
//     city.add_nuclear_plant(2, (1, 3), plant2_supplies, plant2_policy);
//     println!("☢️  Planta Nuclear 2: Zona Este (1, 3) - columna 3");

//     println!("\n✅ ThreadCity configurada exitosamente");
//     println!("   Grid: {}x{} = {} cuadras", 
//              layout.grid_rows, layout.grid_cols, 
//              layout.grid_rows * layout.grid_cols);
//     println!("   Río: Columna {} (vertical)", layout.river_column);
//     println!("   Puentes: 3 (filas {}, {}, {})", 
//              layout.bridge1_row, layout.bridge2_row, layout.bridge3_row);
//     println!("   Comercios: 20");
//     println!("   Plantas: Planta 1 (Oeste, col 1), Planta 2 (Este, col 3)\n");

//     (city, layout)
// }

// pub fn get_bridge_coords(layout: &CityLayout, bridge_id: u32) -> (u32, u32) {
//     let row = match bridge_id {
//         1 => layout.bridge1_row,
//         2 => layout.bridge2_row,
//         3 => layout.bridge3_row,
//         _ => layout.bridge1_row,
//     };
//     (row, layout.river_column)
// }

// pub fn is_west_zone(layout: &CityLayout, col: u32) -> bool {
//     col < layout.river_column
// }

// pub fn is_east_zone(layout: &CityLayout, col: u32) -> bool {
//     col > layout.river_column
// }

// pub fn get_nearest_bridge(layout: &CityLayout, from_row: u32) -> u32 {
//     let dist1 = (from_row as i32 - layout.bridge1_row as i32).abs();
//     let dist2 = (from_row as i32 - layout.bridge2_row as i32).abs();
//     let dist3 = (from_row as i32 - layout.bridge3_row as i32).abs();

//     if dist1 <= dist2 && dist1 <= dist3 {
//         1
//     } else if dist2 <= dist3 {
//         2
//     } else {
//         3
//     }
// }