use gtk::cairo::Context;
use pangocairo::functions::{create_layout, show_layout};
use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// --- Constantes de la Cuadricula y Dibujo (ahora publicas) ---
pub const GRID_ROWS: u32 = 5;
pub const GRID_COLS: u32 = 5;
pub const RIVER_COL: u32 = 2;

pub const BRIDGE_ROWS: [u32; 3] = [2, 3, 4];

pub const COLOR_GRASS: (f64, f64, f64) = (0.4, 0.7, 0.3);
pub const COLOR_ROAD: (f64, f64, f64) = (0.3, 0.3, 0.3);
pub const COLOR_RIVER_TOP: (f64, f64, f64) = (0.3, 0.5, 0.8);
pub const COLOR_RIVER_BOTTOM: (f64, f64, f64) = (0.2, 0.4, 0.7);
pub const COLOR_BRIDGE_BASE: (f64, f64, f64) = (0.25, 0.25, 0.25);
pub const COLOR_BRIDGE_SURFACE: (f64, f64, f64) = (0.4, 0.4, 0.4);
pub const COLOR_PLANT: (f64, f64, f64) = (0.7, 0.7, 0.7);
pub const COLOR_BUILDING_MAIN: (f64, f64, f64) = (0.8, 0.78, 0.75);
pub const COLOR_BUILDING_SHADOW: (f64, f64, f64) = (0.6, 0.58, 0.55);

// colores para entidades
const COLOR_CAR: (f64, f64, f64) = (0.9, 0.2, 0.2);
const COLOR_AMB: (f64, f64, f64) = (1.0, 1.0, 1.0);
const COLOR_TRK: (f64, f64, f64) = (0.9, 0.6, 0.1);
const COLOR_BOT: (f64, f64, f64) = (0.2, 0.6, 0.9);

// --- estado de escena para animar entidades ---

#[derive(Clone, Copy)]
pub enum EntityKind {
    Car,
    Ambulance,
    Boat,
    Truck,
}

#[derive(Clone, Copy)]
pub struct EntityVis {
    pub kind: EntityKind,
    pub pos: (u32, u32),
}

#[derive(Default)]
pub struct SceneState {
    // id -> entidad visible
    pub entities: HashMap<u32, EntityVis>,
}

impl SceneState {
    pub fn set_entity(&mut self, id: u32, kind: EntityKind, pos: (u32, u32)) {
        self.entities.insert(id, EntityVis { kind, pos });
    }

    pub fn move_entity(&mut self, id: u32, to: (u32, u32)) {
        if let Some(e) = self.entities.get_mut(&id) {
            e.pos = to;
        }
    }

    pub fn remove_entity(&mut self, id: u32) {
        self.entities.remove(&id);
    }
}

pub type SharedScene = Rc<RefCell<SceneState>>;

// --- Funciones de Dibujo (ahora publicas) ---

pub fn draw_background_and_roads(cr: &Context, width: i32, height: i32) {
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
        cr.rectangle(
            block_w * (RIVER_COL + 1) as f64,
            y,
            block_w * (GRID_COLS - RIVER_COL - 1) as f64,
            road_w,
        );
        cr.fill().unwrap();
    }

    for i in 1..GRID_COLS {
        if i == RIVER_COL || i == RIVER_COL + 1 {
            continue;
        }
        let x = i as f64 * block_w - road_w / 2.0;
        cr.rectangle(x, 0.0, road_w, height as f64);
        cr.fill().unwrap();
    }
}

pub fn draw_river(cr: &Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let river_x = block_w * RIVER_COL as f64;
    let mut rng = rand::thread_rng();

    let pattern = gtk::cairo::LinearGradient::new(river_x, 0.0, river_x, height as f64);
    pattern.add_color_stop_rgb(0.0, COLOR_RIVER_TOP.0, COLOR_RIVER_TOP.1, COLOR_RIVER_TOP.2);
    pattern.add_color_stop_rgb(1.0, COLOR_RIVER_BOTTOM.0, COLOR_RIVER_BOTTOM.1, COLOR_RIVER_BOTTOM.2);
    cr.set_source(&pattern).unwrap();
    cr.rectangle(river_x, 0.0, block_w, height as f64);
    cr.fill().unwrap();

    cr.set_source_rgba(0.6, 0.8, 1.0, 0.3);
    cr.set_line_width(1.5);
    for i in 0..15 {
        let y_start = i as f64 * (height as f64 / 10.0);
        cr.move_to(river_x, y_start);
        cr.curve_to(
            river_x + block_w / 2.0,
            y_start + 10.0,
            river_x + block_w / 2.0,
            y_start - 10.0,
            river_x + block_w,
            y_start,
        );
        cr.stroke().unwrap();
    }

    cr.set_source_rgba(0.9, 0.95, 1.0, 0.2);
    for _ in 0..150 {
        let rand_x = rng.gen_range(river_x..river_x + block_w);
        let rand_y = rng.gen_range(0.0..height as f64);
        let rand_w = rng.gen_range(1.0..5.0);
        let rand_h = rng.gen_range(1.0..3.0);

        cr.save().unwrap();
        cr.translate(rand_x, rand_y);
        cr.scale(rand_w, rand_h);
        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
        cr.fill().unwrap();
        cr.restore().unwrap();
    }
}

pub fn draw_bridges(cr: &Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;
    
    let bridge_road_indices = BRIDGE_ROWS;

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

pub fn draw_plants(cr: &Context, width: i32, height: i32) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;

    let plant1_pos_grid = (1, 0);
    let plant2_pos_grid = (2, 4);

    let (px1, py1) = (
        (plant1_pos_grid.1 as f64 + 0.5) * block_w,
        (plant1_pos_grid.0 as f64 + 0.5) * block_h,
    );
    draw_single_plant(cr, px1, py1);
    draw_text(cr, px1 - 25.0, py1 + 35.0, "Planta 1");

    let (px2, py2) = (
        (plant2_pos_grid.1 as f64 + 0.5) * block_w,
        (plant2_pos_grid.0 as f64 + 0.5) * block_h,
    );
    draw_single_plant(cr, px2, py2);
    draw_text(cr, px2 - 25.0, py2 + 35.0, "Planta 2");
}

pub fn draw_commerce_buildings(cr: &Context, width: i32, height: i32) {
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

// dibujo de entidades segun escena
pub fn draw_entities(cr: &Context, width: i32, height: i32, scene: &SceneState) {
    let block_w = width as f64 / GRID_COLS as f64;
    let block_h = height as f64 / GRID_ROWS as f64;
    let road_w = 15.0;

    for (_id, ev) in scene.entities.iter() {
        let (row, col) = ev.pos;

        match ev.kind {
            // Los barcos se dibujan en el centro del rio
            EntityKind::Boat => {
                let cx = (RIVER_COL as f64 + 0.5) * block_w;
                let cy = (row as f64 + 0.5) * block_h;
                draw_entity_ellipse(cr, cx, cy, COLOR_BOT);
            }

            // Carro / ambulancia / camion -> sobre las calles
            EntityKind::Car | EntityKind::Ambulance | EntityKind::Truck => {
                // --- EJE Y: elegir una de las 4 calles horizontales reales (1..GRID_ROWS-1) ---
                // row = 0 -> calle 1
                // row = 1 -> calle 2
                // row = 2 -> calle 3
                // row = 3 -> calle 4
                // row = 4 -> tambien calle 4 (no inventamos calle 5)
                let road_row_idx = std::cmp::min(row + 1, GRID_ROWS - 1) as f64;
                let cy = road_row_idx * block_h - road_w / 2.0;

                // --- EJE X: solo hay 2 calles verticales:
                // indice 1 (entre col 0 y 1) -> lado oeste
                // indice 4 (entre col 3 y 4) -> lado este
                let road_col_idx = if col < RIVER_COL {
                    1  // oeste
                } else {
                    GRID_COLS - 1 // este (para cols 3 y 4)
                } as f64;
                let cx = road_col_idx * block_w - road_w / 2.0;

                match ev.kind {
                    EntityKind::Car => draw_entity_rect(cr, cx, cy, COLOR_CAR),
                    EntityKind::Ambulance => draw_entity_cross(cr, cx, cy, COLOR_AMB),
                    EntityKind::Truck => draw_entity_rect(cr, cx, cy, COLOR_TRK),
                    EntityKind::Boat => unreachable!(),
                }
            }
        }
    }
}

// --- Helpers de Dibujo (privados al modulo) ---

fn draw_single_plant(cr: &Context, x: f64, y: f64) {
    cr.set_source_rgb(COLOR_PLANT.0, COLOR_PLANT.1, COLOR_PLANT.2);
    let base_width = 30.0;
    let top_width = 22.0;
    let plant_height = 50.0;

    cr.move_to(x - base_width / 2.0, y + plant_height / 2.0);
    cr.curve_to(
        x - 15.0,
        y,
        x - 15.0,
        y,
        x - top_width / 2.0,
        y - plant_height / 2.0,
    );
    cr.line_to(x + top_width / 2.0, y - plant_height / 2.0);
    cr.curve_to(
        x + 15.0,
        y,
        x + 15.0,
        y,
        x + base_width / 2.0,
        y + plant_height / 2.0,
    );
    cr.close_path();
    cr.fill().unwrap();
}

fn draw_single_building(cr: &Context, x: f64, y: f64) {
    let building_size = 25.0;
    let shadow_offset = 3.0;
    let half_size = building_size / 2.0;

    cr.set_source_rgb(
        COLOR_BUILDING_SHADOW.0,
        COLOR_BUILDING_SHADOW.1,
        COLOR_BUILDING_SHADOW.2,
    );
    cr.rectangle(
        x - half_size + shadow_offset,
        y - half_size + shadow_offset,
        building_size,
        building_size,
    );
    cr.fill().unwrap();

    cr.set_source_rgb(
        COLOR_BUILDING_MAIN.0,
        COLOR_BUILDING_MAIN.1,
        COLOR_BUILDING_MAIN.2,
    );
    cr.rectangle(x - half_size, y - half_size, building_size, building_size);
    cr.fill().unwrap();
}

fn draw_text(cr: &Context, x: f64, y: f64, text: &str) {
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

// helpers para entidades

fn draw_entity_rect(cr: &Context, x: f64, y: f64, color: (f64, f64, f64)) {
    let w = 12.0;
    let h = 8.0;
    cr.set_source_rgb(color.0, color.1, color.2);
    cr.rectangle(x - w / 2.0, y - h / 2.0, w, h);
    cr.fill().unwrap();
}

fn draw_entity_ellipse(cr: &Context, x: f64, y: f64, color: (f64, f64, f64)) {
    cr.set_source_rgb(color.0, color.1, color.2);
    cr.save().unwrap();
    cr.translate(x, y);
    cr.scale(8.0, 5.0);
    cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * std::f64::consts::PI);
    cr.fill().unwrap();
    cr.restore().unwrap();
}

fn draw_entity_cross(cr: &Context, x: f64, y: f64, color: (f64, f64, f64)) {
    cr.set_source_rgb(0.2, 0.2, 0.2);
    cr.rectangle(x - 7.0, y - 5.0, 14.0, 10.0);
    cr.fill().unwrap();
    cr.set_source_rgb(color.0, color.1, color.2);
    cr.rectangle(x - 2.0, y - 5.0, 4.0, 10.0);
    cr.fill().unwrap();
    cr.rectangle(x - 7.0, y - 2.0, 14.0, 4.0);
    cr.fill().unwrap();
}
