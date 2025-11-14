use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CellRendererText, DrawingArea, ListStore,
    Orientation, ScrolledWindow, TextBuffer, TextView, TreeView, TreeViewColumn,
};
use once_cell::sync::OnceCell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

use super::drawing;
use super::drawing::{SceneState, SharedScene};
use crate::ui::event_queue::{EntityKind, EventQueue, UiEvent};
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

    let scene: SharedScene = Rc::new(RefCell::new(SceneState::default()));
    let events = Rc::new(RefCell::new(EventQueue::new()));

    let map = DrawingArea::new();
    map.set_content_width(600);
    map.set_content_height(600);
    {
        let scene_for_draw = scene.clone();
        map.set_draw_func(move |_, cr, width, height| {
            drawing::draw_background_and_roads(cr, width, height);
            drawing::draw_river(cr, width, height);
            drawing::draw_bridges(cr, width, height);
            drawing::draw_plants(cr, width, height);
            drawing::draw_commerce_buildings(cr, width, height);
            drawing::draw_entities(cr, width, height, &scene_for_draw.borrow());
        });
    }

    let text_buffer = TextBuffer::new(None);
    let text_view = TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(gtk::WrapMode::Word)
        .buffer(&text_buffer)
        .build();
    let scrolled_logs = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&text_view)
        .build();

    hbox_top.append(&map);
    hbox_top.append(&scrolled_logs);

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
        col.set_expand(true);
        col.pack_start(&renderer, true);
        col.add_attribute(&renderer, "text", i as i32);
        tree.append_column(&col);
    }
    let scroll_table = ScrolledWindow::builder()
        .height_request(150)
        .child(&tree)
        .build();

    // --- CAMBIO 1: CREAR EL LABEL PARA EL MENSAJE FINAL ---
    let end_message_label = gtk::Label::new(None);
    end_message_label.set_visible(false); // Empezará oculto

    vbox.append(&hbox_top);
    vbox.append(&scroll_table);
    vbox.append(&end_message_label); // Añadimos el label al final del layout
    window.set_child(Some(&vbox));

    let ui_logger = UiLogger::init(text_buffer.clone(), events.clone());
    let (sender, receiver) = mpsc::channel::<String>();
    LOG_SENDER.set(sender).ok();

    {
        let ui_logger = ui_logger.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            for line in receiver.try_iter() {
                ui_logger.log(&line);
            }
            glib::ControlFlow::Continue
        });
    }

    {
        let events = events.clone();
        let scene = scene.clone();
        let map_area = map.clone();
        // --- CAMBIO 2: CLONAR EL LABEL PARA USARLO EN EL CLOSURE ---
        let end_label_clone = end_message_label.clone();

        // --- APLICAMOS EL CAMBIO DE VELOCIDAD AQUÍ ---
        glib::timeout_add_local(std::time::Duration::from_millis(350), move || {
            if let Some(ev) = events.borrow_mut().pop() {
                match ev {
                    UiEvent::Spawn { id, kind, pos } => {
                        let kind_vis = match kind {
                            EntityKind::Car => drawing::EntityKind::Car,
                            EntityKind::Ambulance => drawing::EntityKind::Ambulance,
                            EntityKind::Boat => drawing::EntityKind::Boat,
                            EntityKind::Truck => drawing::EntityKind::Truck,
                        };
                        scene.borrow_mut().set_entity(id, kind_vis, pos);
                        map_area.queue_draw();
                    }
                    UiEvent::Move { id, to } => {
                        scene.borrow_mut().move_entity(id, to);
                        map_area.queue_draw();
                    }
                    UiEvent::Remove { id } => {
                        scene.borrow_mut().remove_entity(id);
                        map_area.queue_draw();
                    }
                    UiEvent::Log(_) => {
                        // no hace nada visual
                    }
                    // --- CAMBIO 3: AÑADIR EL CASO PARA EL NUEVO EVENTO ---
                    UiEvent::SimulationFinished => {
                        let markup = "<span size='xx-large' weight='bold' foreground='lime'>Simulación Finalizada</span>";
                        end_label_clone.set_markup(markup);
                        end_label_clone.set_visible(true);
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    threadcity::set_logger(ui_log_fn);

    thread::spawn(|| {
        threadcity::run_simulation();
    });

    window.show();
}
