//Aquí ocurre la magia: creamos la ventana, establecemos el canal de comunicación y lanzamos la simulación en su propio hilo.

use gtk4 as gtk;
use gtk::{prelude::*, Application, ApplicationWindow, DrawingArea};
use glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

use crate::state::SimulationState;
use crate::drawing;

pub fn build_ui(app: &Application) {
    
}