// Motor de renderizado con Cairo

use cairo::Context;
use gtk::DrawingArea;
use gtk::prelude::*;
use crate::simulation::SimulationState;

const CELL_SIZE: f64 = 80.0;
const MARGIN: f64 = 20.0;

pub fn render_empty(cr: &Context, widget: &DrawingArea) {
    let width = widget.allocated_width() as f64;
    let height = widget.allocated_height() as f64;
    
    // Fondo blanco
    cr.set_source_rgb(0.95, 0.95, 0.95);
    cr.paint().unwrap();
    
    // Mensaje
    cr.set_source_rgb(0.3, 0.3, 0.3);
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(24.0);
    
    let text = "Presiona 'Iniciar' para comenzar la simulación";
    let extents = cr.text_extents(text).unwrap();
    cr.move_to(
        (width - extents.width()) / 2.0,
        height / 2.0
    );
    cr.show_text(text).unwrap();
}

pub fn render_city(cr: &Context, _widget: &DrawingArea, sim: &SimulationState) {
    // Fondo
    cr.set_source_rgb(0.9, 0.95, 0.9);
    cr.paint().unwrap();
    
    let city = sim.city.lock().unwrap();
    let layout = &sim.layout;
    
    // Dibujar grid
    render_grid(cr, layout.grid_rows, layout.grid_cols);
    
    // Dibujar río
    render_river(cr, layout.river_column, layout.grid_rows);
    
    // Dibujar puentes
    render_bridges(cr, layout);
    
    // Dibujar comercios
    render_commerces(cr, &city.commerces);
    
    // Dibujar plantas nucleares
    render_plants(cr, &city.plants);
    
    // Dibujar agentes
    render_agents(cr, sim);
    
    // Dibujar leyenda
    render_legend(cr);
}

fn render_grid(cr: &Context, rows: u32, cols: u32) {
    cr.set_source_rgb(0.7, 0.7, 0.7);
    cr.set_line_width(1.0);
    
    // Líneas verticales
    for col in 0..=cols {
        let x = MARGIN + col as f64 * CELL_SIZE;
        cr.move_to(x, MARGIN);
        cr.line_to(x, MARGIN + rows as f64 * CELL_SIZE);
        cr.stroke().unwrap();
    }
    
    // Líneas horizontales
    for row in 0..=rows {
        let y = MARGIN + row as f64 * CELL_SIZE;
        cr.move_to(MARGIN, y);
        cr.line_to(MARGIN + cols as f64 * CELL_SIZE, y);
        cr.stroke().unwrap();
    }
}

fn render_river(cr: &Context, river_col: u32, rows: u32) {
    let x = MARGIN + river_col as f64 * CELL_SIZE;
    
    // Río azul
    cr.set_source_rgba(0.2, 0.4, 0.8, 0.3);
    cr.rectangle(x, MARGIN, CELL_SIZE, rows as f64 * CELL_SIZE);
    cr.fill().unwrap();
    
    // Ondas del agua (decorativo)
    cr.set_source_rgba(0.3, 0.5, 0.9, 0.4);
    cr.set_line_width(2.0);
    
    for i in 0..10 {
        let y_start = MARGIN + i as f64 * (rows as f64 * CELL_SIZE / 10.0);
        cr.move_to(x + 10.0, y_start);
        cr.curve_to(
            x + 25.0, y_start + 5.0,
            x + 35.0, y_start - 5.0,
            x + 50.0, y_start
        );
        cr.stroke().unwrap();
    }
    
    // Texto "RÍO"
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.select_font_face("Sans", cairo::FontSlant::Italic, cairo::FontWeight::Bold);
    cr.set_font_size(14.0);
    cr.move_to(x + 20.0, MARGIN + 30.0);
    cr.show_text("RÍO").unwrap();
}

fn render_bridges(cr: &Context, layout: &threadcity::CityLayout) {
    let bridges_info = [
        (layout.bridge1_row, "Semáforo"),
        (layout.bridge2_row, "Ceda"),
        (layout.bridge3_row, "Levadizo"),
    ];
    
    for (row, name) in bridges_info {
        let x = MARGIN + layout.river_column as f64 * CELL_SIZE;
        let y = MARGIN + row as f64 * CELL_SIZE;
        
        // Rectángulo del puente
        cr.set_source_rgb(0.5, 0.35, 0.2);
        cr.rectangle(x, y + CELL_SIZE * 0.3, CELL_SIZE, CELL_SIZE * 0.4);
        cr.fill().unwrap();
        
        // Texto
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.0);
        cr.move_to(x + 5.0, y + CELL_SIZE * 0.5);
        cr.show_text(name).unwrap();
    }
}

fn render_commerces(cr: &Context, commerces: &[threadcity::Commerce]) {
    cr.set_source_rgb(0.9, 0.7, 0.3);
    
    for commerce in commerces {
        let x = MARGIN + commerce.location.y as f64 * CELL_SIZE;
        let y = MARGIN + commerce.location.x as f64 * CELL_SIZE;
        
        // Cuadrado pequeño
        cr.rectangle(x + CELL_SIZE * 0.7, y + CELL_SIZE * 0.1, CELL_SIZE * 0.2, CELL_SIZE * 0.2);
        cr.fill().unwrap();
    }
}

fn render_plants(cr: &Context, plants: &[threadcity::NuclearPlant]) {
    for plant in plants {
        let x = MARGIN + plant.loc.y as f64 * CELL_SIZE;
        let y = MARGIN + plant.loc.x as f64 * CELL_SIZE;
        
        // Color según estado
        match plant.status {
            threadcity::PlantStatus::Ok => cr.set_source_rgb(0.0, 0.8, 0.0),
            threadcity::PlantStatus::AtRisk => cr.set_source_rgb(1.0, 0.6, 0.0),
            threadcity::PlantStatus::Exploded => cr.set_source_rgb(1.0, 0.0, 0.0),
        }
        
        // Círculo de la planta
        cr.arc(
            x + CELL_SIZE / 2.0,
            y + CELL_SIZE / 2.0,
            CELL_SIZE * 0.3,
            0.0,
            2.0 * std::f64::consts::PI
        );
        cr.fill().unwrap();
        
        // Símbolo nuclear
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(20.0);
        cr.move_to(x + CELL_SIZE * 0.35, y + CELL_SIZE * 0.55);
        cr.show_text("☢").unwrap();
        
        // ID de la planta
        cr.set_font_size(12.0);
        cr.move_to(x + CELL_SIZE * 0.4, y + CELL_SIZE * 0.8);
        cr.show_text(&format!("P{}", plant.id)).unwrap();
    }
}

fn render_agents(cr: &Context, sim: &SimulationState) {
    let agents = sim.agents.lock().unwrap();
    
    for agent in agents.iter() {
        let x = MARGIN + agent.pos.y as f64 * CELL_SIZE;
        let y = MARGIN + agent.pos.x as f64 * CELL_SIZE;
        
        // Color según tipo
        match agent.agent_type {
            crate::simulation::VisualAgentType::Car => {
                cr.set_source_rgb(0.2, 0.2, 0.8);
            }
            crate::simulation::VisualAgentType::Ambulance => {
                cr.set_source_rgb(1.0, 0.0, 0.0);
            }
            crate::simulation::VisualAgentType::Truck => {
                cr.set_source_rgb(0.6, 0.4, 0.0);
            }
            crate::simulation::VisualAgentType::Boat => {
                cr.set_source_rgb(0.0, 0.5, 0.7);
            }
        }
        
        // Dibujar vehículo como rectángulo (offset para no solapar con plantas)
        let offset_x = if agent.agent_type == crate::simulation::VisualAgentType::Truck {
            CELL_SIZE * 0.15 // Camiones más a la izquierda
        } else {
            CELL_SIZE * 0.3
        };
        
        let offset_y = if agent.agent_type == crate::simulation::VisualAgentType::Truck {
            CELL_SIZE * 0.15
        } else {
            CELL_SIZE * 0.3
        };
        
        cr.rectangle(
            x + offset_x,
            y + offset_y,
            CELL_SIZE * 0.35,
            CELL_SIZE * 0.35
        );
        cr.fill().unwrap();
        
        // ID del agente
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(9.0);
        cr.move_to(x + offset_x + 5.0, y + offset_y + 15.0);
        cr.show_text(&format!("{}", agent.id)).unwrap();
    }
}

fn render_legend(cr: &Context) {
    let legend_x = MARGIN;
    let legend_y = MARGIN + 6.0 * CELL_SIZE + 20.0;
    
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.rectangle(legend_x, legend_y, 400.0, 150.0);
    cr.fill().unwrap();
    
    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.set_line_width(2.0);
    cr.rectangle(legend_x, legend_y, 400.0, 150.0);
    cr.stroke().unwrap();
    
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(14.0);
    cr.move_to(legend_x + 10.0, legend_y + 20.0);
    cr.show_text("Leyenda de Agentes:").unwrap();
    
    let items = [
        (0.2, 0.2, 0.8, "Carros (Azul)"),
        (1.0, 0.0, 0.0, "Ambulancias (Rojo)"),
        (0.6, 0.4, 0.0, "Camiones (Café)"),
        (0.0, 0.5, 0.7, "Barcos (Azul oscuro)"),
    ];
    
    cr.set_font_size(12.0);
    for (i, (r, g, b, text)) in items.iter().enumerate() {
        let y_offset = legend_y + 45.0 + i as f64 * 22.0;
        
        // Cuadrado de color más grande
        cr.set_source_rgb(*r, *g, *b);
        cr.rectangle(legend_x + 15.0, y_offset - 12.0, 18.0, 18.0);
        cr.fill().unwrap();
        
        // Texto
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.move_to(legend_x + 40.0, y_offset);
        cr.show_text(text).unwrap();
    }
    
    // Añadir info de comercios y plantas
    cr.set_font_size(11.0);
    cr.set_source_rgb(0.4, 0.4, 0.4);
    cr.move_to(legend_x + 10.0, legend_y + 135.0);
    cr.show_text("🟧 Comercios (cuadrados naranjas) | ☢ Plantas nucleares").unwrap();
}