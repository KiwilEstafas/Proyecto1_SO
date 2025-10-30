use mypthreads::mutex::MyMutex;
use mypthreads::{my_mutex_lock, my_mutex_unlock};
use mypthreads::signals::ThreadSignal;
use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::ThreadId;

struct BridgeState {
    vehicles_on_bridge: u32,
    is_ship_passing: bool,
    // Cola con prioridad: (prioridad, id_del_hilo)
    waiting_threads: Vec<(u8, ThreadId)>,
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

    //Solicitud de paso de un vehículo con prioridad
    pub fn request_pass_vehicle(
        &mut self,
        rt: &mut ThreadRuntime,
        priority: u8,
    ) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block;
        }

        if self.state.is_ship_passing || self.state.vehicles_on_bridge >= self.lanes as u32 {
            println!(
                "[Puente {}] Vehículo (prio {}) no puede pasar. Esperando... (Ocupación: {}, Barco: {})",
                self.id, priority, self.state.vehicles_on_bridge, self.state.is_ship_passing
            );

            if let Some(tid) = rt.current() {
                // Evitar duplicados
                if !self.state.waiting_threads.iter().any(|&(_, waiting_tid)| waiting_tid == tid) {
                    self.state.waiting_threads.push((priority, tid));
                    println!(
                        "   [Puente {}] Hilo {} (prio {}) agregado a cola de espera",
                        self.id, tid, priority
                    );
                }
            }

            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }

        // Vehículo puede entrar
        self.state.vehicles_on_bridge += 1;
        println!(
            "[Puente {}] Vehículo (prio {}) entrando. Ocupación: {}",
            self.id, priority, self.state.vehicles_on_bridge
        );

        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    // Liberar el paso de un vehículo y despertar al siguiente más prioritario
    pub fn release_pass_vehicle(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        if self.state.vehicles_on_bridge > 0 {
            self.state.vehicles_on_bridge -= 1;
        }
        println!(
            "[Puente {}] Vehículo saliendo. Ocupación: {}",
            self.id, self.state.vehicles_on_bridge
        );

        let mut thread_to_wake: Option<ThreadId> = None;

        if !self.state.waiting_threads.is_empty() {
            // Buscar el hilo con mayor prioridad
            let mut max_prio = 0;
            let mut best_idx = 0;

            for (i, &(prio, _)) in self.state.waiting_threads.iter().enumerate() {
                if prio > max_prio {
                    max_prio = prio;
                    best_idx = i;
                }
            }

            // Sacar ese hilo de la cola
            let (_, tid) = self.state.waiting_threads.remove(best_idx);
            thread_to_wake = Some(tid);

            println!(
                "   [Puente {}] Despertando hilo {} (prio {}) para cruzar",
                self.id, tid, max_prio
            );
        }

        my_mutex_unlock(rt, &mut self.mutex);

        // Despertar fuera del mutex
        if let Some(tid) = thread_to_wake {
            rt.wake(tid);
        }
    }

    // Solicitud de paso de un barco
    pub fn request_pass_boat(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
        let signal = my_mutex_lock(rt, &mut self.mutex);
        if signal == ThreadSignal::Block {
            return ThreadSignal::Block;
        }

        if self.state.vehicles_on_bridge > 0 || self.state.is_ship_passing {
            println!(
                "[Puente {}] Barco no puede pasar (Vehículos: {}, Otro Barco: {}). Esperando...",
                self.id, self.state.vehicles_on_bridge, self.state.is_ship_passing
            );

            if let Some(tid) = rt.current() {
                // Usamos prioridad 0 para barcos (puede ajustarse si querés priorizarlos)
                if !self.state.waiting_threads.iter().any(|&(_, waiting_tid)| waiting_tid == tid) {
                    self.state.waiting_threads.push((0, tid));
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

    // Fin del paso del barco
    pub fn release_pass_boat(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        self.state.is_ship_passing = false;
        println!("[Puente {}] Barco ha pasado y cierra el canal.", self.id);

        // Despertar al hilo más prioritario luego del barco
        let mut thread_to_wake: Option<ThreadId> = None;

        if !self.state.waiting_threads.is_empty() {
            let mut max_prio = 0;
            let mut best_idx = 0;

            for (i, &(prio, _)) in self.state.waiting_threads.iter().enumerate() {
                if prio > max_prio {
                    max_prio = prio;
                    best_idx = i;
                }
            }

            let (_, tid) = self.state.waiting_threads.remove(best_idx);
            thread_to_wake = Some(tid);
            println!(
                "   [Puente {}] Despertando hilo {} (prio {}) tras paso del barco",
                self.id, tid, max_prio
            );
        }

        my_mutex_unlock(rt, &mut self.mutex);

        if let Some(tid) = thread_to_wake {
            rt.wake(tid);
        }
    }
}
