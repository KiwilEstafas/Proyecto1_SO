use std::rc::Rc;
use std::cell::RefCell;

// Importa los componentes de tu simulación del crate `threadcity`
use threadcity::sim::City;
use threadcity::agents::{Agent, AgentDowncast, Car, Ambulance, Boat, CargoTruck};
use threadcity::model::{Coord, DeadlinePolicy, SupplyKind, SupplySpec};

// --- ¡IMPORTACIONES CLAVE DE TU BIBLIOTECA MYPTHREADS! ---
use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::{SchedulerType, ThreadEntry};
use mypthreads::api_rust::*; // Importamos las funciones amigables como my_thread_create

fn main() {
    println!("--- Iniciando simulación de ThreadCity con mypthreads ---");

    // ===================================================================
    // PASO 1: CREAR EL RUNTIME Y EL ESTADO COMPARTIDO DE LA CIUDAD
    // ===================================================================
    // El ThreadRuntime será el "kernel" que gestiona todos nuestros hilos-agentes.
    let mut runtime = ThreadRuntime::new();

    // Creamos la ciudad base. Ya no contendrá a los agentes directamente.
    // Se convierte en el "mundo" que los hilos observarán y modificarán.
    let city = City::new(20, 30); // grid 20x30
    
    // Envolvemos la ciudad en `Rc<RefCell<T>>` para poder compartirla de forma
    // segura y mutable entre todos nuestros hilos cooperativos.
    let shared_city = Rc::new(RefCell::new(city));

    // Añadimos puentes y otros elementos al estado compartido de la ciudad.
    {
        let mut city_ref = shared_city.borrow_mut();
        city_ref.add_commerce(1, (5, 5));
        city_ref.add_bridge(1, 2); // Puente con ID 1 y 2 carriles
    }


    // ===================================================================
    // PASO 2: CREAR LOS AGENTES Y "SPAWNEARLOS" COMO MYPTHREADS
    // ===================================================================
    // Ya no hacemos `city.add_agent(...)`. En su lugar, por cada agente,
    // creamos un mypthread que se encargará de su ciclo de vida.

    // Definimos una lista de agentes que queremos en la simulación.
    let agents_to_spawn: Vec<Box<dyn AgentDowncast + Send>> = vec![
        Box::new(Car::new(100, (0, 9), (10, 9))),
        Box::new(Car::new(101, (1, 9), (12, 9))),
        Box::new(Ambulance::new(200, (1, 0), (19, 12))),
        Box::new(Boat::new(300, (5, 0), (5, 19))),
    ];

    for mut agent in agents_to_spawn {
        let city_clone = shared_city.clone();
        
        // Creamos un nombre para el hilo y lo clonamos para evitar problemas de ownership.
        let agent_name = format!("Agent-{}", agent.id());
        let thread_name_for_creation = agent_name.clone();

        let agent_logic: ThreadEntry = Box::new(move |rt, _| {
            // --- ¡ESTA ES LA "VIDA" DE UN AGENTE! ---
            
            // Lógica de interacción con el puente (ejemplo para coches)
            // Asumimos que el río está en la columna Y=10 y el puente conecta Y=9 con Y=11
            if let Some(car) = agent.as_any().downcast_ref::<Car>() {
                 if car.pos().y == 9 {
                     println!("[{}] en la entrada del puente (pos: {:?}). Intentando cruzar...", agent_name, car.pos());
                     
                     // Pedimos prestado el estado de la ciudad para interactuar con él
                     let mut city = city_clone.borrow_mut();
                     let bridge = &mut city.bridges[0];

                     // Esta es la interacción clave: el hilo intenta usar un recurso compartido.
                     // Si el puente está lleno o bloqueado, esta llamada devolverá `Block`,
                     // y el runtime pondrá este hilo a dormir.
                     return bridge.request_pass_vehicle(rt);
                 }
            }
            
            // El agente se mueve un paso si no está interactuando con nada.
            agent.step(100); 

            // (Aquí iría la lógica para liberar el puente una vez cruzado)

            // Si no hemos hecho nada que nos bloquee o nos termine, cedemos el control.
            my_thread_yield()
        });

        // Creamos el hilo en nuestro runtime.
        my_thread_create(&mut runtime, &thread_name_for_creation, SchedulerType::RoundRobin, agent_logic, None, None);
    }


    // ===================================================================
    // PASO 3: EL BUCLE PRINCIPAL DE LA SIMULACIÓN
    // ===================================================================
    // Este bucle reemplaza tu antiguo `for tick in 0..10`.
    // Ahora, en cada "tick", ejecutamos un paso de UN solo hilo.
    let mut tick = 0;
    const MAX_TICKS: u32 = 500;

    println!("\n--- Corriendo simulación ---");
    while !runtime.ready.is_empty() && tick < MAX_TICKS {
        // Ejecuta el siguiente hilo en la cola 'ready' según el scheduler.
        runtime.run_once();
        
        // Avanzamos el reloj lógico de la simulación.
        runtime.advance_time(10); // Ej: 10ms por cada acción de un agente

        if tick % 25 == 0 {
             println!("\nTick: {}, Hilos activos: {}, Tiempo Sim: {}ms", tick, runtime.ready.len(), runtime.now());
             // Opcional: Imprimir el estado de los agentes
             // (Para esto, los agentes deberían estar en un estado compartido también)
        }

        tick += 1;
        // std::thread::sleep(std::time::Duration::from_millis(10)); // Descomentar para ralentizar la simulación
    }

    println!("\n--- Simulación finalizada en el tick {} ---", tick);
    println!("Estado final de la cola 'ready': {}", runtime.ready.len());
}