// binario de ejemplo para correr la simulacion base

use threadcity::sim::City;
use threadcity::agents::{Car, Ambulance, Boat, CargoTruck};
use threadcity::model::{SupplySpec, SupplyKind, DeadlinePolicy};

fn main() {
    println!("threadcity base");

    // construir ciudad base
    let mut city = City::new(20, 30); // grid 20x30
    city.add_commerce(1, (5, 5));
    city.add_commerce(2, (12, 7));
    city.add_bridge(1, 3); // puente con 3 carriles logicos
    city.add_river();
    city.add_nuclear_plant(
        1,
        vec![
            SupplySpec { kind: SupplyKind::Water, deadline_ms: 5_000, period_ms: 10_000 },
            SupplySpec { kind: SupplyKind::Radioactive, deadline_ms: 2_000, period_ms: 5_000 },
        ],
        DeadlinePolicy { max_lateness_ms: 2_000 },
    );

    // agregar agentes iniciales
    city.add_agent(Box::new(Car::new(100, (0, 0), (10, 10))));
    city.add_agent(Box::new(Ambulance::new(200, (1, 0), (19, 12))));
    city.add_agent(Box::new(Boat::new(300, (0, 15), (19, 15))));
    city.add_agent(Box::new(CargoTruck::new(400, (2, 2), (5, 5), SupplyKind::Water)));

    // loop simple de simulacion en un solo hilo
    for tick in 0..10 {
        city.step(1_000); // 1s de tiempo logico
        println!("tick {} now={}ms", tick, city.now());
    }

    // ejemplo opcional de concurrencia real con std::thread
    // esto crea un par de agentes en hilos del so para probar aislado
    city.run_agents_in_threads_for_demo(3, 500);
    println!("listo");
}

