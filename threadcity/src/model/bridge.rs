// threadcity/src/model/bridge.rs

use mypthreads::mutex::MyMutex;
use mypthreads::{my_mutex_lock, my_mutex_unlock};
use mypthreads::signals::ThreadSignal;
use mypthreads::runtime::ThreadRuntime;
use mypthreads::thread::ThreadId;

/// Define la dirección del tráfico para controlar los puentes.
/// Asumimos que el río es vertical, por lo que el tráfico es Este/Oeste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    WestToEast,
    EastToWest,
}

/// Contiene la lógica y el estado específico para cada tipo de puente.
#[derive(Debug)]
pub enum BridgeLogic {
    /// Puente 1: Semáforo que alterna entre direcciones.
    TrafficLight {
        lanes: u8,
        current_direction: TrafficDirection,
        green_duration_ms: u64,
        time_in_current_state_ms: u64,
    },
    /// Puente 2: Ceda el paso, una dirección tiene prioridad.
    Yield {
        lanes: u8,
        priority_direction: TrafficDirection,
    },
    /// Puente 3: Levadizo, permite el paso de barcos.
    Drawbridge {
        lanes: u8,
    },
}

/// Estado interno y compartido del puente, protegido por un mutex.
#[derive(Debug, Default)]
struct BridgeState {
    vehicles_on_bridge: u32,
    // Qué dirección de tráfico está usando el puente actualmente
    direction_on_bridge: Option<TrafficDirection>,
    is_boat_passing: bool,
    // La cola de espera ahora incluye la dirección deseada por cada hilo
    waiting_threads: Vec<(u8, ThreadId, TrafficDirection)>,
}


pub struct Bridge {
    pub id: u32,
    logic: BridgeLogic, // El comportamiento específico del puente
    mutex: MyMutex,
    state: BridgeState,
}

impl Bridge {
    // --- Constructores para cada tipo de puente ---

    pub fn new_traffic_light(id: u32, lanes: u8, green_duration_ms: u64) -> Self {
        Self {
            id,
            logic: BridgeLogic::TrafficLight {
                lanes,
                current_direction: TrafficDirection::WestToEast, // Empieza en una dirección
                green_duration_ms,
                time_in_current_state_ms: 0,
            },
            mutex: MyMutex::my_mutex_init(),
            state: BridgeState::default(),
        }
    }

    pub fn new_yield(id: u32, lanes: u8, priority_direction: TrafficDirection) -> Self {
        Self {
            id,
            logic: BridgeLogic::Yield { lanes, priority_direction },
            mutex: MyMutex::my_mutex_init(),
            state: BridgeState::default(),
        }
    }

    pub fn new_drawbridge(id: u32, lanes: u8) -> Self {
        Self {
            id,
            logic: BridgeLogic::Drawbridge { lanes },
            mutex: MyMutex::my_mutex_init(),
            state: BridgeState::default(),
        }
    }

    /// Método llamado en cada tick de la simulación para actualizar estados internos (ej: semáforos)
    pub fn step(&mut self, dt_ms: u64, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        if let BridgeLogic::TrafficLight {
            ref mut current_direction,
            green_duration_ms,
            ref mut time_in_current_state_ms,
            ..
        } = self.logic
        {
            *time_in_current_state_ms += dt_ms;
            if *time_in_current_state_ms >= green_duration_ms {
                // Cambiar de dirección el semáforo
                *time_in_current_state_ms = 0;
                *current_direction = match *current_direction {
                    TrafficDirection::WestToEast => TrafficDirection::EastToWest,
                    TrafficDirection::EastToWest => TrafficDirection::WestToEast,
                };
                println!("[Puente Semáforo {}] LUZ VERDE para la dirección {:?}", self.id, *current_direction);

                // Despertar a los hilos que esperaban por esta dirección
                let (threads_to_wake, remaining_threads) = self.state.waiting_threads.drain(..).partition(|(_, _, dir)| *dir == *current_direction);
                self.state.waiting_threads = remaining_threads;

                my_mutex_unlock(rt, &mut self.mutex); // Liberar mutex ANTES de despertar
                for (_, tid, _) in threads_to_wake {
                    rt.wake(tid);
                }
                return; // Salimos para no desbloquear el mutex dos veces
            }
        }
        my_mutex_unlock(rt, &mut self.mutex);
    }


    /// Un vehículo solicita pasar por el puente
    pub fn request_pass_vehicle(
        &mut self,
        rt: &mut ThreadRuntime,
        priority: u8,
        direction: TrafficDirection,
    ) -> ThreadSignal {
        if my_mutex_lock(rt, &mut self.mutex) == ThreadSignal::Block {
            return ThreadSignal::Block;
        }

        // --- Lógica de decisión para permitir o bloquear ---
        let can_pass = match &self.logic {
            BridgeLogic::TrafficLight { lanes, current_direction, .. } => {
                let light_is_green = *current_direction == direction;
                let bridge_is_free = self.state.vehicles_on_bridge == 0;
                let bridge_is_same_dir = self.state.direction_on_bridge == Some(direction);

                // Condiciones para pasar:
                // 1. La luz está verde para mi dirección Y hay espacio.
                // 2. O, el puente está ocupado por vehículos en mi misma dirección (para terminar de vaciarlo).
                !self.state.is_boat_passing &&
                self.state.vehicles_on_bridge < *lanes as u32 &&
                (light_is_green || (bridge_is_same_dir && !bridge_is_free))
            }

            BridgeLogic::Yield { lanes, priority_direction } => {
                let bridge_is_free = self.state.vehicles_on_bridge == 0;
                let bridge_is_same_dir = self.state.direction_on_bridge == Some(direction);
                let other_direction_is_waiting = self.state.waiting_threads.iter().any(|(_, _, dir)| *dir != direction);

                // Condiciones para pasar:
                // 1. El puente está libre Y nadie de la dirección prioritaria está esperando.
                // 2. O, mi dirección es la prioritaria Y el puente está libre.
                // 3. O, ya hay gente pasando en mi dirección y hay espacio.
                 !self.state.is_boat_passing &&
                 self.state.vehicles_on_bridge < *lanes as u32 &&
                 (
                    (bridge_is_free && (direction == *priority_direction || !other_direction_is_waiting)) ||
                    bridge_is_same_dir
                 )
            }

            BridgeLogic::Drawbridge { lanes } => {
                let bridge_is_free = self.state.vehicles_on_bridge == 0;
                let bridge_is_same_dir = self.state.direction_on_bridge == Some(direction);

                // Condición: No está pasando un barco Y (el puente está libre O va en mi dirección) Y hay espacio.
                !self.state.is_boat_passing &&
                self.state.vehicles_on_bridge < *lanes as u32 &&
                (bridge_is_free || bridge_is_same_dir)
            }
        };

        if can_pass {
            self.state.vehicles_on_bridge += 1;
            self.state.direction_on_bridge = Some(direction);
            println!("[Puente {}] Vehículo (prio {}, dir {:?}) ENTRANDO. Ocupación: {}", self.id, priority, direction, self.state.vehicles_on_bridge);
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Continue;
        }

        // --- Si no puede pasar, se añade a la cola de espera ---
        println!("[Puente {}] Vehículo (prio {}, dir {:?}) BLOQUEADO. A la cola de espera.", self.id, priority, direction);
        if let Some(tid) = rt.current() {
            if !self.state.waiting_threads.iter().any(|&(_, waiting_tid, _)| waiting_tid == tid) {
                self.state.waiting_threads.push((priority, tid, direction));
                // Mantenemos la cola ordenada por prioridad (mayor a menor)
                self.state.waiting_threads.sort_by_key(|(p, _, _)| std::cmp::Reverse(*p));
            }
        }
        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Block
    }

    /// Un vehículo notifica que ha terminado de cruzar
    pub fn release_pass_vehicle(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);

        if self.state.vehicles_on_bridge > 0 {
            self.state.vehicles_on_bridge -= 1;
        }
        println!("[Puente {}] Vehículo SALIENDO. Ocupación: {}", self.id, self.state.vehicles_on_bridge);
        
        let mut thread_to_wake: Option<ThreadId> = None;

        // Si el puente queda vacío, reseteamos la dirección
        if self.state.vehicles_on_bridge == 0 {
            self.state.direction_on_bridge = None;
            println!("[Puente {}] Puente ahora VACÍO.", self.id);
        }

        // --- Lógica para despertar al siguiente hilo ---
        // Buscamos en la cola de espera el hilo más prioritario que AHORA SÍ PUEDA PASAR.
        let mut best_idx_to_wake: Option<usize> = None;
        for (i, &(_, _, dir)) in self.state.waiting_threads.iter().enumerate() {
            // Re-evaluamos si este hilo puede pasar bajo las condiciones actuales
             let can_pass_now = match &self.logic {
                BridgeLogic::TrafficLight { current_direction, .. } => *current_direction == dir,
                BridgeLogic::Yield { priority_direction, .. } => {
                    let other_dir_waiting = self.state.waiting_threads.iter().any(|(_,_,d)| *d != dir);
                    dir == *priority_direction || !other_dir_waiting
                },
                BridgeLogic::Drawbridge { .. } => !self.state.is_boat_passing,
            };

            if can_pass_now {
                best_idx_to_wake = Some(i);
                break; // Encontramos al mejor candidato (la lista ya está ordenada por prioridad)
            }
        }

        if let Some(idx) = best_idx_to_wake {
            let (_, tid, dir) = self.state.waiting_threads.remove(idx);
            thread_to_wake = Some(tid);
            println!("[Puente {}] Despertando hilo {} (dir {:?}) de la cola.", self.id, tid, dir);
        }

        my_mutex_unlock(rt, &mut self.mutex);

        if let Some(tid) = thread_to_wake {
            rt.wake(tid);
        }
    }

    // Lógica para barcos (aplica principalmente al Drawbridge)
    // Se puede mantener similar, ya que es más simple: o pasa o no.
    pub fn request_pass_boat(&mut self, rt: &mut ThreadRuntime) -> ThreadSignal {
         if my_mutex_lock(rt, &mut self.mutex) == ThreadSignal::Block { return ThreadSignal::Block; }

        if self.state.vehicles_on_bridge > 0 || self.state.is_boat_passing {
            // Bloqueado, pero no lo metemos a la cola para no complicar el ejemplo.
            // Una implementación más robusta tendría una cola para barcos.
            my_mutex_unlock(rt, &mut self.mutex);
            return ThreadSignal::Block;
        }

        self.state.is_boat_passing = true;
        println!("[Puente Levadizo {}] BARCO pasando. Bloqueando tráfico de vehículos.", self.id);
        my_mutex_unlock(rt, &mut self.mutex);
        ThreadSignal::Continue
    }

    pub fn release_pass_boat(&mut self, rt: &mut ThreadRuntime) {
        my_mutex_lock(rt, &mut self.mutex);
        self.state.is_boat_passing = false;
        println!("[Puente Levadizo {}] BARCO ha pasado. Liberando tráfico.", self.id);

        // Despertamos a todos los vehículos para que re-intenten cruzar
        let threads_to_wake = self.state.waiting_threads.drain(..).collect::<Vec<_>>();
        my_mutex_unlock(rt, &mut self.mutex);

        for (_, tid, _) in threads_to_wake {
            rt.wake(tid);
        }
    }
}