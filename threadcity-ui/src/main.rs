use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Orientation, TextView, TextBuffer, DrawingArea, ScrolledWindow,
    Box as GtkBox, ListStore, TreeView, TreeViewColumn, CellRendererText,
};
use once_cell::sync::OnceCell;
use std::thread;
use std::sync::mpsc;
use pangocairo::functions::{create_layout, show_layout};
use rand::Rng;

mod ui_logger;
use ui_logger::UiLogger;

// --- Constantes de la Cuadrícula y Dibujo ---
const GRID_ROWS: u32 = 5;
const GRID_COLS: u32 = 5;
const RIVER_COL: u32 = 2;

const COLOR_GRASS: (f64, f64, f64) = (0.4, 0.7, 0.3);
const COLOR_ROAD: (f64, f64, f64) = (0.3, 0.3, 0.3);
const COLOR_RIVER_TOP: (f64, f64, f64) = (0.3, 0.5, 0.8);
const COLOR_RIVER_BOTTOM: (f64, f64, f64) = (0.2, 0.4, 0.7);
const COLOR_BRIDGE_BASE: (f64, f64, f64) = (0.25, 0.25, 0.25);
const COLOR_BRIDGE_SURFACE: (f64, f64, f64) = (0.4, 0.4, 0.4);
const COLOR_PLANT: (f64, f64, f64) = (0.7, 0.7, 0.7);
const COLOR_BUILDING_MAIN: (f64, f64, f64) = (0.8, 0.78, 0.75);
const COLOR_BUILDING_SHADOW: (f64, f64, f64) = (0.6, 0.58, 0.55);

static LOG_SENDER: OnceCell<mpsc::Sender<String>> = OnceCell::new();

fn ui_log_fn(msg: &str) {
    if let Some(tx) = LOG_SENDER.get() {
        let _ = tx.send(msg.to_string());
    }
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
            .default_width(1000)
            .default_height(800)
            .build();

        let vbox = GtkBox::new(Orientation::Vertical, 5);
        let hbox_top = GtkBox::new(Orientation::Horizontal, 5);

        let map = DrawingArea::new();
        map.set_content_width(600);
        map.set_content_height(600);
        map.set_draw_func(|_, cr, width, height| {
            draw_background_and_roads(cr, width, height);
            draw_river(cr, width, height);
            draw_bridges(cr, width, height);
            draw_plants(cr, width, height);
            draw_commerce_buildings(cr, width, height);
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
    });

    app.run();
}

// --- FUNCIONES DE DIBUJO ---

fn draw_background_and_roads(cr: &gtk::cairo::Context, width: i32, height: i32) {
    cr.set_source_rgb(COLOR_GRASS.0, COLOR_GRASS.1, COLOR_GRASS.2);
    cr.paint().unwrap();

    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;
    let road_w = 15.0;

    cr.set_source_rgb(COLOR_ROAD.0, COLOR_ROAD.1, COLOR_ROAD.2);

    for i in 1..GRID_ROWS {
        let y = i as f64 * block_h - road_w / 2.0;
        cr.rectangle(0.0, y, block_w * RIVER_COL as f64, road_w);
        cr.fill().unwrap();
        cr.rectangle(block_w * (RIVER_COL + 1) as f64, y, block_w * (GRID_COLS - RIVER_COL - 1) as f64, road_w);
        cr.fill().unwrap();
    }

    for i in 1..GRID_COLS {
        if i == RIVER_COL || i == RIVER_COL + 1 { continue; }
        let x = i as f64 * block_w - road_w / 2.0;
        cr.rectangle(x, 0.0, road_w, height as f64);
        cr.fill().unwrap();
    }
}

fn draw_river(cr: &gtk::cairo::Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let river_x = block_w * RIVER_COL as f64;
    let mut rng = rand::rng();

    // 1. Base del río con un gradiente
    // <-- CAMBIO: Se usa la ruta correcta `gtk::cairo::LinearGradient`
    let pattern = gtk::cairo::LinearGradient::new(river_x, 0.0, river_x, height as f64);
    pattern.add_color_stop_rgb(0.0, COLOR_RIVER_TOP.0, COLOR_RIVER_TOP.1, COLOR_RIVER_TOP.2);
    pattern.add_color_stop_rgb(1.0, COLOR_RIVER_BOTTOM.0, COLOR_RIVER_BOTTOM.1, COLOR_RIVER_BOTTOM.2);
    cr.set_source(&pattern).unwrap();
    cr.rectangle(river_x, 0.0, block_w, height as f64);
    cr.fill().unwrap();

    // 2. Ondas principales
    cr.set_source_rgba(0.6, 0.8, 1.0, 0.3);
    cr.set_line_width(1.5);
    for i in 0..15 {
        let y_start = i as f64 * (height as f64 / 10.0);
        cr.move_to(river_x, y_start);
        cr.curve_to(river_x + block_w / 2.0, y_start + 10.0, river_x + block_w / 2.0, y_start - 10.0, river_x + block_w, y_start);
        cr.stroke().unwrap();
    }
    
    // 3. Textura de reflejos de luz
    cr.set_source_rgba(0.9, 0.95, 1.0, 0.2);
    for _ in 0..150 {
        let rand_x = rng.random_range(river_x..river_x + block_w);
        let rand_y = rng.random_range(0.0..height as f64);
        let rand_w = rng.random_range(1.0..5.0);
        let rand_h = rng.random_range(1.0..3.0);

        cr.save().unwrap();
        cr.translate(rand_x, rand_y);
        cr.scale(rand_w, rand_h);
        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
        cr.fill().unwrap();
        cr.restore().unwrap();
    }
}

fn draw_bridges(cr: &gtk::cairo::Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;
    
    let bridge_road_indices = [1, 2, 4];

    for (i, road_num) in bridge_road_indices.iter().enumerate() {
        let y_center = *road_num as f64 * block_h;
        let x_start = block_w * RIVER_COL as f64;
        
        let bridge_height = 25.0;
        let surface_height = 15.0;

        cr.set_source_rgb(COLOR_BRIDGE_BASE.0, COLOR_BRIDGE_BASE.1, COLOR_BRIDGE_BASE.2);
        cr.rectangle(x_start, y_center - bridge_height / 2.0, block_w, bridge_height);
        cr.fill().unwrap();

        cr.set_source_rgb(COLOR_BRIDGE_SURFACE.0, COLOR_BRIDGE_SURFACE.1, COLOR_BRIDGE_SURFACE.2);
        cr.rectangle(x_start, y_center - surface_height / 2.0, block_w, surface_height);
        cr.fill().unwrap();

        cr.set_source_rgb(1.0, 0.8, 0.2);
        cr.set_dash(&[6.0, 4.0], 0.0);
        cr.set_line_width(2.0);
        cr.move_to(x_start, y_center);
        cr.line_to(x_start + block_w, y_center);
        cr.stroke().unwrap();
        cr.set_dash(&[], 0.0);

        draw_text(cr, x_start + 5.0, y_center + 15.0, &format!("Puente {}", i + 1));
    }
}

fn draw_plants(cr: &gtk::cairo::Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;

    let plant1_pos_grid = (1, 0);
    let plant2_pos_grid = (2, 4);

    let (px1, py1) = ((plant1_pos_grid.1 as f64 + 0.5) * block_w, (plant1_pos_grid.0 as f64 + 0.5) * block_h);
    draw_single_plant(cr, px1, py1);
    draw_text(cr, px1 - 25.0, py1 + 35.0, "Planta 1");

    let (px2, py2) = ((plant2_pos_grid.1 as f64 + 0.5) * block_w, (plant2_pos_grid.0 as f64 + 0.5) * block_h);
    draw_single_plant(cr, px2, py2);
    draw_text(cr, px2 - 25.0, py2 + 35.0, "Planta 2");
}

fn draw_commerce_buildings(cr: &gtk::cairo::Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;
    
    let plant1_pos = (1, 0);
    let plant2_pos = (2, 4);

    for r in 0..GRID_ROWS {
        for c in 0..GRID_COLS {
            let current_pos = (r, c);
            if c == RIVER_COL || current_pos == plant1_pos || current_pos == plant2_pos {
                continue;
            }
            
            let cx = (c as f64 + 0.5) * block_w;
            let cy = (r as f64 + 0.5) * block_h;
            
            if (r == 2 && c == 1) || (r == 2 && c == 3) {
                 draw_single_building(cr, cx - 15.0, cy);
                 draw_single_building(cr, cx + 15.0, cy);
            } else {
                 draw_single_building(cr, cx, cy);
            }
        }
    }
}

// --- HELPERS DE DIBUJO INDIVIDUAL ---

fn draw_single_plant(cr: &gtk::cairo::Context, x: f64, y: f64) {
    cr.set_source_rgb(COLOR_PLANT.0, COLOR_PLANT.1, COLOR_PLANT.2);
    let base_width = 30.0;
    let top_width = 22.0;
    let plant_height = 50.0;
    
    cr.move_to(x - base_width / 2.0, y + plant_height / 2.0);
    cr.curve_to(x - 15.0, y, x - 15.0, y, x - top_width / 2.0, y - plant_height / 2.0);
    cr.line_to(x + top_width / 2.0, y - plant_height / 2.0);
    cr.curve_to(x + 15.0, y, x + 15.0, y, x + base_width / 2.0, y + plant_height / 2.0);
    cr.close_path();
    cr.fill().unwrap();
}

fn draw_single_building(cr: &gtk::cairo::Context, x: f64, y: f64) {
    let building_size = 25.0;
    let shadow_offset = 3.0;
    let half_size = building_size / 2.0;

    cr.set_source_rgb(COLOR_BUILDING_SHADOW.0, COLOR_BUILDING_SHADOW.1, COLOR_BUILDING_SHADOW.2);
    cr.rectangle(x - half_size + shadow_offset, y - half_size + shadow_offset, building_size, building_size);
    cr.fill().unwrap();

    cr.set_source_rgb(COLOR_BUILDING_MAIN.0, COLOR_BUILDING_MAIN.1, COLOR_BUILDING_MAIN.2);
    cr.rectangle(x - half_size, y - half_size, building_size, building_size);
    cr.fill().unwrap();
}

fn draw_text(cr: &gtk::cairo::Context, x: f64, y: f64, text: &str) {
    let layout = create_layout(cr);
    let mut desc = pango::FontDescription::new();
    desc.set_family("Sans");
    desc.set_weight(pango::Weight::Bold);
    desc.set_size(10 * pango::SCALE);
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    cr.move_to(x, y);
    cr.set_source_rgb(0.1, 0.1, 0.1);
    show_layout(cr, &layout);
}