use mypthreads::mutex::MyMutex;
use mypthreads::{my_mutex_lock, my_mutex_unlock};
use mypthreads::signals::ThreadSignal;
use mypthreads::runtime::ThreadRuntime;

struct BridgeState {
    vehicles_on_bridge: u32,
    is_ship_passing: bool,
}

pub struct Bridge {
    pub id: u32,
    lanes: u8, // Capacidad total de vehículos
    mutex: MyMutex,
    state: BridgeState,
}

impl Bridge {
    pub fn new(id: u32, lanes: u8) -> Self {
        Self {
            id,
            lanes,
            mutex: MyMutex::my_mutex_init(),
            state: BridgeState {
                vehicles_on_bridge: 0,
                is_ship_passing: false,
            },
        }
    }

    // Un VEHÍCULO (hilo) intenta cruzar.
    pub fn request_pass_vehicle(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block; 
        }

        // Si tenemos el lock, evaluamos el estado.
        // CONDICIÓN 1: No podemos pasar si un barco está pasando.
        // CONDICIÓN 2: No podemos pasar si el puente está lleno.
        if self.state.is_ship_passing || self.state.vehicles_on_bridge >= self.lanes as u32 {
            println!("[Puente {}] Vehículo no puede pasar (Barco: {}, Ocupación: {}). Esperando...", 
                     self.id, self.state.is_ship_passing, self.state.vehicles_on_bridge);
            // Liberamos el lock y bloqueamos para que otros puedan actuar.
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }

        self.state.vehicles_on_bridge += 1;
        println!("[Puente {}] Vehículo entrando. Ocupación: {}", self.id, self.state.vehicles_on_bridge);

        // Liberamos el lock y continuamos.
        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    // Un VEHÍCULO (hilo) ha terminado de cruzar.
    pub fn release_pass_vehicle(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex); // Obtener lock para modificar estado
        
        if self.state.vehicles_on_bridge > 0 {
            self.state.vehicles_on_bridge -= 1;
        }
        println!("[Puente {}] Vehículo saliendo. Ocupación: {}", self.id, self.state.vehicles_on_bridge);

        my_mutex_unlock(rt, &mut self.mutex);
    }

    // Un BARCO (hilo) intenta cruzar. 
    pub fn request_pass_boat(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block;
        }

        // CONDICIÓN 1: No podemos pasar si hay vehículos en el puente.
        // CONDICIÓN 2: No podemos pasar si otro barco ya está pasando.
        if self.state.vehicles_on_bridge > 0 || self.state.is_ship_passing {
            println!("[Puente {}] Barco no puede pasar (Vehículos: {}, Otro Barco: {}). Esperando...", 
                     self.id, self.state.vehicles_on_bridge, self.state.is_ship_passing);
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }
        
        // Abrimos el canal.
        self.state.is_ship_passing = true;
        println!("[Puente {}] Barco abriendo canal y pasando.", self.id);

        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    // Un BARCO (hilo) ha terminado de cruzar. 
    pub fn release_pass_boat(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        self.state.is_ship_passing = false;
        println!("[Puente {}] Barco ha pasado y cierra el canal.", self.id);
        my_mutex_unlock(rt, &mut self.mutex);
    }
}