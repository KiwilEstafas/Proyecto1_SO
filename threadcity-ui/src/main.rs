
// Declaramos los nuevos módulos que vamos a crear
mod app;
mod state;
mod drawing;

use gtk4 as gtk;
use gtk::{prelude::*, Application};

fn main() {
    let app = Application::builder()
        .application_id("com.threadcity.simulator")
        .build();

    // Conectamos la función que construye la UI al evento "activate"
    app.connect_activate(app::build_ui);

    // Corremos la aplicación
    app.run();
}