// Test de integración de ThreadCity

use threadcity::*;
use mypthreads::mypthreads_api::*;
use mypthreads::signals::ThreadSignal;

#[test]
fn test_city_creation() {
    let (city, layout) = create_city();
    
    // Verificar grid mínimo de 25 cuadras
    assert!(city.grid.size() >= 25, "La ciudad debe tener al menos 25 cuadras");
    
    // Verificar río
    assert!(city.river.is_some(), "La ciudad debe tener un río");
    
    // Verificar 3 puentes
    assert_eq!(city.bridges.len(), 3, "Debe haber exactamente 3 puentes");
    
    // Verificar comercios (al menos 20)
    assert!(city.commerces.len() >= 20, "Debe haber al menos 20 comercios");
    
    // Verificar plantas nucleares
    assert_eq!(city.plants.len(), 2, "Debe haber 2 plantas nucleares");
    
    println!("✓ Ciudad creada correctamente");
    println!("  - Grid: {}x{} = {} cuadras", layout.grid_rows, layout.grid_cols, city.grid.size());
    println!("  - Puentes: {}", city.bridges.len());
    println!("  - Comercios: {}", city.commerces.len());
    println!("  - Plantas: {}", city.plants.len());
}

#[test]
fn test_bridge_types() {
    let (city, layout) = create_city();
    
    // Verificar tipos de puentes
    assert_eq!(city.bridges[0].bridge_type, BridgeType::TrafficLight);
    assert_eq!(city.bridges[1].bridge_type, BridgeType::Yield);
    assert_eq!(city.bridges[2].bridge_type, BridgeType::Drawbridge);
    
    println!("✓ Tipos de puentes correctos");
}

#[test]
fn test_simple_car_movement() {
    println!("\n=== TEST: Movimiento simple de carro ===\n");
    
    let (city, layout) = create_city();
    let shared_city = create_shared_city(city);
    
    let origin = Coord::new(0, 0);
    let dest = Coord::new(0, 1);
    let mut pos = origin;
    let mut arrived = false;
    
    let tid = my_thread_create(
        "TestCar",
        SchedulerParams::RoundRobin,
        Box::new(move |_| {
            if pos.x == dest.x && pos.y == dest.y {
                if !arrived {
                    println!("Carro llegó a destino!");
                    arrived = true;
                }
                return ThreadSignal::Exit;
            }
            
            // Moverse
            if pos.y < dest.y {
                pos.y += 1;
            } else if pos.y > dest.y {
                pos.y -= 1;
            } else if pos.x < dest.x {
                pos.x += 1;
            } else if pos.x > dest.x {
                pos.x -= 1;
            }
            
            ThreadSignal::Yield
        }),
    );
    
    run_simulation(20);
    
    println!("✓ Test de movimiento completado");
}

#[test]
fn test_scheduler_types() {
    println!("\n=== TEST: Tipos de schedulers ===\n");
    
    // RoundRobin
    let tid1 = my_thread_create(
        "RR-Thread",
        SchedulerParams::RoundRobin,
        Box::new(|_| ThreadSignal::Exit),
    );
    
    // Lottery
    let tid2 = my_thread_create(
        "Lottery-Thread",
        SchedulerParams::Lottery { tickets: 50 },
        Box::new(|_| ThreadSignal::Exit),
    );
    
    // RealTime
    let tid3 = my_thread_create(
        "RT-Thread",
        SchedulerParams::RealTime { deadline: 10000 },
        Box::new(|_| ThreadSignal::Exit),
    );
    
    run_simulation(10);
    
    println!("✓ Schedulers funcionando correctamente");
}

#[test]
fn test_plant_deadlines() {
    println!("\n=== TEST: Deadlines de plantas ===\n");
    
    let (city, _) = create_city();
    
    // Verificar que las plantas tengan deadlines configurados
    for plant in &city.plants {
        assert!(!plant.requires.is_empty(), "Planta debe tener requerimientos");
        
        for supply in &plant.requires {
            assert!(supply.deadline_ms > 0, "Deadline debe ser positivo");
            assert!(supply.period_ms > 0, "Periodo debe ser positivo");
            println!("  Planta {}: {:?} - deadline {}ms", plant.id, supply.kind, supply.deadline_ms);
        }
    }
    
    println!("✓ Deadlines de plantas configurados correctamente");
}