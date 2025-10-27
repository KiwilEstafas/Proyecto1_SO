use mypthreads::mutex::MyMutex;
use mypthreads::{my_mutex_lock, my_mutex_unlock};
use mypthreads::signals::ThreadSignal;
use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::ThreadId;

struct BridgeState {
    vehicles_on_bridge: u32,
    is_ship_passing: bool,
    waiting_threads: Vec<ThreadId>,
}

pub struct Bridge {
    pub id: u32,
    lanes: u8,
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
                waiting_threads: Vec::new(),
            },
        }
    }

    pub fn request_pass_vehicle(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block; 
        }

        if self.state.is_ship_passing || self.state.vehicles_on_bridge >= self.lanes as u32 {
            println!("[Puente {}] Vehículo no puede pasar (Barco: {}, Ocupación: {}). Esperando...", 
                     self.id, self.state.is_ship_passing, self.state.vehicles_on_bridge);
            
            // guardar el hilo actual en la lista de espera
            if let Some(tid) = rt.current() {
                if !self.state.waiting_threads.contains(&tid) {
                    self.state.waiting_threads.push(tid);
                    println!("   [Puente {}] Hilo {} agregado a cola de espera", self.id, tid);
                }
            }
            
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }

        self.state.vehicles_on_bridge += 1;
        println!("[Puente {}] Vehículo entrando. Ocupación: {}", self.id, self.state.vehicles_on_bridge);

        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    pub fn release_pass_vehicle(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);
        
        if self.state.vehicles_on_bridge > 0 {
            self.state.vehicles_on_bridge -= 1;
        }
        println!("[Puente {}] Vehículo saliendo. Ocupación: {}", self.id, self.state.vehicles_on_bridge);

        // Despertar a los hilos que estaban esperando
        let threads_to_wake = std::mem::take(&mut self.state.waiting_threads);
        
        my_mutex_unlock(rt, &mut self.mutex);
        
        for tid in threads_to_wake {
            println!("   [Puente {}] Despertando hilo {}", self.id, tid);
            rt.wake(tid);
        }
    }

    pub fn request_pass_boat(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block;
        }

        if self.state.vehicles_on_bridge > 0 || self.state.is_ship_passing {
            println!("[Puente {}] Barco no puede pasar (Vehículos: {}, Otro Barco: {}). Esperando...", 
                     self.id, self.state.vehicles_on_bridge, self.state.is_ship_passing);
            
            if let Some(tid) = rt.current() {
                if !self.state.waiting_threads.contains(&tid) {
                    self.state.waiting_threads.push(tid);
                }
            }
            
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }
        
        self.state.is_ship_passing = true;
        println!("[Puente {}] Barco abriendo canal y pasando.", self.id);

        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    pub fn release_pass_boat(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        self.state.is_ship_passing = false;
        println!("[Puente {}] Barco ha pasado y cierra el canal.", self.id);
        
        let threads_to_wake = std::mem::take(&mut self.state.waiting_threads);
        
        my_mutex_unlock(rt, &mut self.mutex);
        
        for tid in threads_to_wake {
            rt.wake(tid);
        }
    }
}