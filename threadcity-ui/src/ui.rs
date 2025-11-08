// Componentes de la interfaz de usuario

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, DrawingArea, Label, Orientation, ScrolledWindow, TextView};
use glib::ControlFlow;
use std::sync::{Arc, Mutex};
use crate::simulation::SimulationState;
use crate::renderer;

pub struct UIState {
    pub simulation: Arc<Mutex<Option<SimulationState>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub speed: Arc<Mutex<u32>>, // ms por ciclo
}

impl UIState {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            simulation: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            speed: Arc::new(Mutex::new(100)),
        }))
    }
}

pub fn create_ui(ui_state: Arc<Mutex<UIState>>) -> GtkBox {
    // Container principal (vertical)
    let main_box = GtkBox::new(Orientation::Vertical, 5);
    main_box.set_margin_start(10);
    main_box.set_margin_end(10);
    main_box.set_margin_top(10);
    main_box.set_margin_bottom(10);

    // === TÍTULO ===
    let title = Label::new(Some("ThreadCity - Simulación con MyPthreads"));
    title.set_markup("<span size='x-large' weight='bold'>ThreadCity - Simulación con MyPthreads</span>");
    main_box.pack_start(&title, false, false, 10);

    // === CONTROLES ===
    let controls = create_controls(Arc::clone(&ui_state));
    main_box.pack_start(&controls, false, false, 5);

    // === ÁREA DE CONTENIDO (Horizontal) ===
    let content_box = GtkBox::new(Orientation::Horizontal, 10);

    // Panel izquierdo: Canvas de simulación
    let canvas_frame = create_canvas(Arc::clone(&ui_state));
    content_box.pack_start(&canvas_frame, true, true, 0);

    // Panel derecho: Información
    let info_panel = create_info_panel(Arc::clone(&ui_state));
    content_box.pack_start(&info_panel, false, false, 0);

    main_box.pack_start(&content_box, true, true, 0);

    // === BARRA DE ESTADO ===
    let status = Label::new(Some("Estado: Detenido | Hilos: 0 | Tiempo: 0ms"));
    status.set_halign(gtk::Align::Start);
    main_box.pack_start(&status, false, false, 5);

    main_box
}

fn create_controls(ui_state: Arc<Mutex<UIState>>) -> GtkBox {
    let controls = GtkBox::new(Orientation::Horizontal, 5);

    // Botón: Iniciar/Detener
    let btn_start = Button::with_label("▶ Iniciar");
    let ui_clone = Arc::clone(&ui_state);
    btn_start.connect_clicked(move |btn| {
        let state = ui_clone.lock().unwrap();
        let mut is_running = state.is_running.lock().unwrap();
        
        if *is_running {
            *is_running = false;
            btn.set_label("▶ Iniciar");
            println!("⏸️  Simulación pausada");
        } else {
            *is_running = true;
            btn.set_label("⏸ Pausar");
            println!("▶️  Simulación iniciada");
            
            // Iniciar simulación si no existe
            let mut sim = state.simulation.lock().unwrap();
            if sim.is_none() {
                *sim = Some(SimulationState::new());
                println!("🏙️  Ciudad creada");
            }
        }
    });
    controls.pack_start(&btn_start, false, false, 5);

    // Botón: Reset
    let btn_reset = Button::with_label("🔄 Reiniciar");
    let ui_clone = Arc::clone(&ui_state);
    btn_reset.connect_clicked(move |_| {
        let state = ui_clone.lock().unwrap();
        *state.is_running.lock().unwrap() = false;
        *state.simulation.lock().unwrap() = None;
        println!("🔄 Simulación reiniciada");
    });
    controls.pack_start(&btn_reset, false, false, 5);

    // Botón: Paso a paso
    let btn_step = Button::with_label("⏭ Paso");
    let ui_clone = Arc::clone(&ui_state);
    btn_step.connect_clicked(move |_| {
        let state = ui_clone.lock().unwrap();
        let mut sim_lock = state.simulation.lock().unwrap();
        if let Some(ref mut sim) = *sim_lock {
            sim.step();
            println!("⏭️  Ejecutando un ciclo");
        }
    });
    controls.pack_start(&btn_step, false, false, 5);

    // Label: Velocidad
    let speed_label = Label::new(Some("Velocidad:"));
    controls.pack_start(&speed_label, false, false, 10);

    // Botones de velocidad
    let speeds = vec![
        ("🐢 Lento", 200),
        ("🚶 Normal", 100),
        ("🏃 Rápido", 50),
        ("⚡ Muy Rápido", 10),
    ];

    for (label, ms) in speeds {
        let btn = Button::with_label(label);
        let ui_clone = Arc::clone(&ui_state);
        btn.connect_clicked(move |_| {
            let state = ui_clone.lock().unwrap();
            *state.speed.lock().unwrap() = ms;
            println!("⏱️  Velocidad: {}ms/ciclo", ms);
        });
        controls.pack_start(&btn, false, false, 2);
    }

    controls
}

fn create_canvas(ui_state: Arc<Mutex<UIState>>) -> ScrolledWindow {
    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    
    // Área de dibujo
    let drawing_area = DrawingArea::new();
    drawing_area.set_size_request(800, 600);
    
    // Conectar evento de dibujo
    let ui_clone = Arc::clone(&ui_state);
    drawing_area.connect_draw(move |widget, cr| {
        // Evitar locks anidados: clonar Arc y luego lockear ese Arc
        let sim_arc = { ui_clone.lock().unwrap().simulation.clone() };
        if let Some(ref sim) = *sim_arc.lock().unwrap() {
            renderer::render_city(cr, widget, sim);
        } else {
            renderer::render_empty(cr, widget);
        }
        glib::Propagation::Proceed
    });

    // Timer para actualizar el canvas
    let ui_clone = Arc::clone(&ui_state);
    let drawing_clone = drawing_area.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        {
            let state = ui_clone.lock().unwrap();
            let is_running = *state.is_running.lock().unwrap();
            
            if is_running {
                // Actualizar simulación
                let mut sim_lock = state.simulation.lock().unwrap();
                if let Some(ref mut sim) = *sim_lock {
                    sim.step();
                }
                
                // Redibujar
                drawing_clone.queue_draw();
            }
        }
        ControlFlow::Continue
    });

    scrolled.add(&drawing_area);
    scrolled
}

fn create_info_panel(_ui_state: Arc<Mutex<UIState>>) -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 5);
    panel.set_size_request(300, -1);

    // Título
    let title = Label::new(Some("Información"));
    title.set_markup("<b>Información de la Simulación</b>");
    panel.pack_start(&title, false, false, 5);

    // Área de texto con scroll
    let scrolled = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    
    let text_view = TextView::new();
    text_view.set_editable(false);
    text_view.set_wrap_mode(gtk::WrapMode::Word);
    
    let buffer = text_view.buffer().unwrap();
    buffer.set_text(
        "🏙️ ThreadCity\n\n\
         Elementos:\n\
         • Grid: 5×5 = 25 cuadras\n\
         • Río: Columna 2 (vertical)\n\
         • Puentes: 3\n\
         • Comercios: 20\n\
         • Plantas: 2\n\n\
         Agentes:\n\
         🚗 Carros (Lottery)\n\
         🚑 Ambulancias (Prioridad)\n\
         🚚 Camiones (RealTime)\n\
         ⛵ Barcos (RoundRobin)\n\n\
         Presiona ▶ Iniciar para comenzar"
    );
    
    scrolled.add(&text_view);
    panel.pack_start(&scrolled, true, true, 0);

    // Estadísticas
    let stats_label = Label::new(Some(""));
    stats_label.set_markup(
        "<b>Estadísticas:</b>\n\
         Ciclos: 0\n\
         Hilos activos: 0\n\
         Entregas: 0/4"
    );
    stats_label.set_halign(gtk::Align::Start);
    panel.pack_start(&stats_label, false, false, 5);

    panel
}