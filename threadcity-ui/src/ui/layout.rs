use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Orientation, TextView, TextBuffer, DrawingArea, ScrolledWindow,
    Box as GtkBox, ListStore, TreeView, TreeViewColumn, CellRendererText,
};
use once_cell::sync::OnceCell;
use std::thread;
use std::sync::mpsc;
use super::drawing;
use crate::ui_logger::UiLogger;

// El logger global ahora vive aquí, encapsulado dentro del módulo de UI.
static LOG_SENDER: OnceCell<mpsc::Sender<String>> = OnceCell::new();

fn ui_log_fn(msg: &str) {
    if let Some(tx) = LOG_SENDER.get() {
        let _ = tx.send(msg.to_string());
    }
    println!("{}", msg);
}

// Función principal que construye toda la interfaz de usuario.
pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("ThreadCity Visualizer")
        .default_width(1000)
        .default_height(800)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 5);
    let hbox_top = GtkBox::new(Orientation::Horizontal, 5);

    // Configurar el DrawingArea para usar nuestras funciones de dibujo
    let map = DrawingArea::new();
    map.set_content_width(600);
    map.set_content_height(600);
    map.set_draw_func(|_, cr, width, height| {
        drawing::draw_background_and_roads(cr, width, height);
        drawing::draw_river(cr, width, height);
        drawing::draw_bridges(cr, width, height);
        drawing::draw_plants(cr, width, height);
        drawing::draw_commerce_buildings(cr, width, height);
    });

    let text_buffer = TextBuffer::new(None);
    let text_view = TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::Word).buffer(&text_buffer).build();
    let scrolled_logs = ScrolledWindow::builder().vexpand(true).hexpand(true).child(&text_view).build();

    hbox_top.append(&map);
    hbox_top.append(&scrolled_logs);

    let columns = ["Entidad", "Estado", "Detalle"];
    let store = ListStore::new(&[String::static_type(), String::static_type(), String::static_type()]);
    let tree = TreeView::with_model(&store);
    for (i, title) in columns.iter().enumerate() {
        let renderer = CellRendererText::new();
        let col = TreeViewColumn::new();
        col.set_title(title);
        col.set_expand(true);
        col.pack_start(&renderer, true);
        col.add_attribute(&renderer, "text", i as i32);
        tree.append_column(&col);
    }
    let scroll_table = ScrolledWindow::builder().height_request(150).child(&tree).build();

    vbox.append(&hbox_top);
    vbox.append(&scroll_table);
    window.set_child(Some(&vbox));

    // --- Lógica de la aplicación ---
    let ui_logger = UiLogger::init(text_buffer.clone());
    let (sender, receiver) = mpsc::channel::<String>();
    LOG_SENDER.set(sender).ok();

    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        for line in receiver.try_iter() {
            ui_logger.log(&line);
        }
        glib::ControlFlow::Continue
    });

    threadcity::set_logger(ui_log_fn);

    thread::spawn(|| {
        threadcity::run_simulation();
    });

    window.show();
}