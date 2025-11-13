mod ui;
mod ui_logger;

use gtk::prelude::*;
use gtk::Application;

fn main() {
    mypthreads::mypthreads_api::runtime_init();

    let app = Application::builder()
        .application_id("com.threadcity.ui")
        .build();

    app.connect_activate(ui::layout::build_ui);
    app.run();
}
