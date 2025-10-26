// puente con control de paso muy simple
// se modela con un contador de cupos y un flag de canal para barcos

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Bridge {
    pub id: u32,
    lanes: Arc<Mutex<u8>>,
    ship_channel_open: Arc<Mutex<bool>>,
}

impl Bridge {
    pub fn new(id: u32, lanes: u8) -> Self {
        Self {
            id,
            lanes: Arc::new(Mutex::new(lanes)),
            ship_channel_open: Arc::new(Mutex::new(true)),
        }
    }

    pub fn request_pass_vehicle(&self) -> bool {
        let mut n = self.lanes.lock().unwrap();
        if *n > 0 {
            *n -= 1;
            true
        } else {
            false
        }
    }

    pub fn release_pass_vehicle(&self) {
        let mut n = self.lanes.lock().unwrap();
        *n += 1;
    }

    pub fn open_ship_channel(&self) {
        *self.ship_channel_open.lock().unwrap() = true;
    }

    pub fn close_ship_channel(&self) {
        *self.ship_channel_open.lock().unwrap() = false;
    }

    pub fn ship_channel_is_open(&self) -> bool {
        *self.ship_channel_open.lock().unwrap()
    }
}

