// Interfaz gráfica GTK para ThreadCity

mod ui;
mod renderer;
mod simulation;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use std::sync::{Arc, Mutex};

const APP_ID: &str = "com.threadcity.ui";

fn main() {
    // Inicializar GTK
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    
    // Ejecutar aplicación
    app.run();
}

fn build_ui(app: &Application) {
    println!("🎨 Iniciando ThreadCity UI con GTK...");
    
    // Crear ventana principal
    let window = ApplicationWindow::builder()
        .application(app)
        .title("ThreadCity - Simulación con Hilos Preemptivos")
        .default_width(1200)
        .default_height(800)
        .build();

    // Crear UI
    let ui_state = ui::UIState::new();
    let ui = ui::create_ui(Arc::clone(&ui_state));
    
    window.add(&ui);
    window.show_all();
    
    println!("✅ Interfaz GTK inicializada");
}
