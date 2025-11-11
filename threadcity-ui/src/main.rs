use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Orientation, TextView, TextBuffer, DrawingArea, ScrolledWindow,
    Box as GtkBox, ListStore, TreeView, TreeViewColumn, CellRendererText,
};
use once_cell::sync::OnceCell;
use std::thread;
use std::sync::mpsc; // Importar el canal estándar de Rust

mod ui_logger;
use ui_logger::UiLogger;

// Canal global para el logger, ahora usando el Sender de mpsc.
static LOG_SENDER: OnceCell<mpsc::Sender<String>> = OnceCell::new();

// Función que instalará threadcity como logger de la UI
fn ui_log_fn(msg: &str) {
    if let Some(tx) = LOG_SENDER.get() {
        let _ = tx.send(msg.to_string());
    }
    // Opcional: mantener consola
    println!("{}", msg);
}

fn main() {
    let app = Application::builder()
        .application_id("com.threadcity.ui")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("ThreadCity Visualizer")
            .default_width(900)
            .default_height(700)
            .build();

        let vbox = GtkBox::new(Orientation::Vertical, 5);
        let hbox_top = GtkBox::new(Orientation::Horizontal, 5);

        // === Mapa estático ===
        let map = DrawingArea::new();
        map.set_content_width(600);
        map.set_content_height(400);
        map.set_draw_func(|_, cr, width, height| {
            cr.set_source_rgb(0.85, 1.0, 0.85);
            cr.paint().unwrap();

            cr.set_source_rgb(0.2, 0.4, 0.9);
            let river_x = width as f64 / 2.0;
            cr.rectangle(river_x - 20.0, 0.0, 40.0, height as f64);
            cr.fill().unwrap();

            cr.set_source_rgb(0.4, 0.4, 0.4);
            let bridge_rows = [80.0, 160.0, 240.0];
            for y in bridge_rows.iter() {
                cr.rectangle(river_x - 40.0, *y, 80.0, 20.0);
                cr.fill().unwrap();
            }

            cr.set_source_rgb(0.9, 0.2, 0.2);
            cr.arc(100.0, 80.0, 15.0, 0.0, std::f64::consts::PI * 2.0);
            cr.fill().unwrap();
            cr.arc(width as f64 - 100.0, 160.0, 15.0, 0.0, std::f64::consts::PI * 2.0);
            cr.fill().unwrap();

            cr.set_source_rgb(1.0, 0.9, 0.2);
            for row in 0..5 {
                for col in 0..5 {
                    if col == 2 { continue; }
                    let x = 50.0 + col as f64 * 100.0;
                    let y = 300.0 + row as f64 * 50.0;
                    cr.rectangle(x, y, 20.0, 20.0);
                    cr.fill().unwrap();
                }
            }
        });

        // === Terminal (logs) ===
        let text_buffer = TextBuffer::new(None);
        let text_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::Word)
            .buffer(&text_buffer)
            .build();
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&text_view)
            .build();

        hbox_top.append(&map);
        hbox_top.append(&scrolled);

        // === Tabla inferior ===
        let columns = ["Entidad", "Estado", "Detalle"];
        let store = ListStore::new(&[
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let tree = TreeView::with_model(&store);
        for (i, title) in columns.iter().enumerate() {
            let renderer = CellRendererText::new();
            let col = TreeViewColumn::new();
            col.set_title(title);
            col.pack_start(&renderer, true);
            col.add_attribute(&renderer, "text", i as i32);
            tree.append_column(&col);
        }
        let scroll_table = ScrolledWindow::builder().vexpand(true).child(&tree).build();

        vbox.append(&hbox_top);
        vbox.append(&scroll_table);
        window.set_child(Some(&vbox));

        // === Canal para recibir logs desde cualquier hilo usando std::sync::mpsc ===
        let ui_logger = UiLogger::init(text_buffer.clone());
        let (sender, receiver) = mpsc::channel::<String>();
        LOG_SENDER.set(sender).ok(); // instala sender global

        // Se sondea el receptor desde el hilo principal de glib cada 50ms
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            // Se drenan todos los mensajes pendientes en el canal
            for line in receiver.try_iter() {
                ui_logger.log(&line);
            }
            glib::ControlFlow::Continue
        });

        // Configura el logger del engine para que escriba a nuestro canal:
        threadcity::set_logger(ui_log_fn);

        // (Opcional) llena la tabla de ejemplo inicial
        let data = vec![
            ("Puente 1", "Verde", "Semáforo activo"),
            ("Planta Oeste", "OK", "Suministro normal"),
            ("Planta Este", "AtRisk", "Esperando agua"),
        ];
        for (ent, est, det) in data {
            store.insert_with_values(None, &[(0, &ent), (1, &est), (2, &det)]);
        }

        // === Lanza la simulación en un hilo aparte del proceso del UI ===
        thread::spawn(|| {
            threadcity::run_simulation();
        });

        window.show();
    });

    app.run();
}