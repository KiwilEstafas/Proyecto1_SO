use gtk::prelude::*;
use gtk::TextBuffer;
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::drawing::{BRIDGE_ROWS, RIVER_COL};
use crate::ui::event_queue::{EntityKind, EventQueue, UiEvent};

// logger del ui que tambien traduce algunos logs a eventos para animacion

#[derive(Clone)]
pub struct UiLogger {
    buffer: Rc<RefCell<TextBuffer>>,
    events: Rc<RefCell<EventQueue>>,
}

impl UiLogger {
    pub fn init(buffer: TextBuffer, events: Rc<RefCell<EventQueue>>) -> Self {
        Self {
            buffer: Rc::new(RefCell::new(buffer)),
            events,
        }
    }

    pub fn log(&self, msg: &str) {
        // escribe en el panel de logs
        let buffer_ref = self.buffer.borrow();
        let mut iter = buffer_ref.end_iter();
        buffer_ref.insert(&mut iter, &format!("{}\n", msg));
        println!("{}", msg);

        // intenta generar eventos de animacion a partir del texto
        self.try_parse_and_enqueue(msg);
    }

    fn try_parse_and_enqueue(&self, line: &str) {
        // --- REGLAS DE CREACIÓN (SPAWN) ---

        if let Some(id) = extract_id(line, "Carro-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn {
                    id,
                    kind: EntityKind::Car,
                    pos: o,
                });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        if let Some(id) = extract_id(line, "Ambulancia-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn {
                    id,
                    kind: EntityKind::Ambulance,
                    pos: o,
                });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        if let Some(id) = extract_id(line, "CargoTruck-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn {
                    id,
                    kind: EntityKind::Truck,
                    pos: o,
                });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        if let Some(id) = extract_id(line, "Barco-") {
            if let Some((origin, destination)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn {
                    id,
                    kind: EntityKind::Boat,
                    pos: origin,
                });
                enqueue_full_boat_path_upwards(&mut *q, id, origin, destination);
                return;
            }
        }

        // --- REGLAS PARA PLANTAS ---

        // Explosión de planta
        if line.contains("¡EXPLOSIÓN! Planta") {
            if let Some(pid) = extract_id(line, "Planta ") {
                self.events
                    .borrow_mut()
                    .push(UiEvent::PlantExploded { id: pid });
            }
            return;
        }

        // Recuperación / reset de planta
        if line.contains("Planta") && line.contains("reiniciándose") {
            if let Some(pid) = extract_id(line, "Planta ") {
                self.events
                    .borrow_mut()
                    .push(UiEvent::PlantRecovered { id: pid });
            }
            return;
        }

        // --- REGLAS DE ELIMINACIÓN PARA VEHÍCULOS QUE NO SON BARCOS ---

        if line.contains("✅ Llegó a destino") {
            if let Some(id) = extract_bracket_id(line) {
                if id < 300 {
                    self.events.borrow_mut().push(UiEvent::Remove { id });
                }
                return;
            }
        }

        // Fallback para cualquier otro log que no coincida con lo anterior.
        self.events
            .borrow_mut()
            .push(UiEvent::Log(line.to_string()));
    }
}

// helpers de parseo

fn extract_id(s: &str, prefix: &str) -> Option<u32> {
    if let Some(i) = s.find(prefix) {
        let rest = &s[i + prefix.len()..];
        let mut num = String::new();
        for ch in rest.chars() {
            if ch.is_ascii_digit() {
                num.push(ch);
            } else {
                break;
            }
        }
        return num.parse::<u32>().ok();
    }
    None
}

fn extract_bracket_id(s: &str) -> Option<u32> {
    if let Some(open) = s.find('[') {
        if let Some(close) = s[open + 1..].find(']') {
            let inside = &s[open + 1..open + 1 + close];
            return inside.trim().parse::<u32>().ok();
        }
    }
    None
}

fn extract_single_coord(s: &str) -> Option<(u32, u32)> {
    if let Some((x, y)) = extract_coord_from(s) {
        return Some((x, y));
    }
    None
}

fn extract_origin_dest(s: &str) -> Option<((u32, u32), (u32, u32))> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some((x, y)) = extract_coord_from(rest) {
        out.push((x, y));
        if let Some(pos) = rest.find("y:") {
            rest = &rest[pos + 2..];
        } else {
            break;
        }
        if out.len() == 2 {
            break;
        }
    }
    if out.len() == 2 {
        Some((out[0], out[1]))
    } else {
        None
    }
}

fn extract_coord_from(s: &str) -> Option<(u32, u32)> {
    let xi = s.find("x: ")?;
    let after_x = &s[xi + 3..];
    let xnum: String = after_x.chars().take_while(|c| c.is_ascii_digit()).collect();
    let x = xnum.parse::<u32>().ok()?;
    let yi = s.find("y: ")?;
    let after_y = &s[yi + 3..];
    let ynum: String = after_y.chars().take_while(|c| c.is_ascii_digit()).collect();
    let y = ynum.parse::<u32>().ok()?;
    Some((x, y))
}

// genera pasos de movimiento manhattan simples
fn enqueue_path(q: &mut EventQueue, id: u32, mut cur: (u32, u32), dest: (u32, u32)) {
    let (dest_row, dest_col) = dest;

    let needs_cross =
        (cur.1 < RIVER_COL && dest_col > RIVER_COL) || (cur.1 > RIVER_COL && dest_col < RIVER_COL);

    if needs_cross {
        let bridge_rows = BRIDGE_ROWS;

        let mut cross_row = bridge_rows[0];
        let mut best = cur.0.abs_diff(bridge_rows[0]);
        for &br in &bridge_rows[1..] {
            let d = cur.0.abs_diff(br);
            if d < best {
                best = d;
                cross_row = br;
            }
        }

        while cur.0 != cross_row {
            if cur.0 < cross_row {
                cur.0 += 1;
            } else {
                cur.0 -= 1;
            }
            q.push(UiEvent::Move { id, to: cur });
        }

        while cur.1 != dest_col {
            if cur.1 < dest_col {
                if cur.1 + 1 == RIVER_COL {
                    cur.1 += 2;
                } else {
                    cur.1 += 1;
                }
            } else {
                if cur.1 - 1 == RIVER_COL {
                    if cur.1 > 0 {
                        cur.1 -= 2;
                    } else {
                        break;
                    }
                } else {
                    cur.1 -= 1;
                }
            }
            q.push(UiEvent::Move { id, to: cur });
        }

        while cur.0 != dest_row {
            if cur.0 < dest_row {
                cur.0 += 1;
            } else {
                cur.0 -= 1;
            }
            q.push(UiEvent::Move { id, to: cur });
        }
    } else {
        while cur.1 != dest_col {
            if cur.1 < dest_col {
                cur.1 += 1;
            } else {
                if cur.1 > 0 {
                    cur.1 -= 1;
                } else {
                    break;
                }
            }
            q.push(UiEvent::Move { id, to: cur });
        }
        while cur.0 != dest_row {
            if cur.0 < dest_row {
                cur.0 += 1;
            } else {
                cur.0 -= 1;
            }
            q.push(UiEvent::Move { id, to: cur });
        }
    }
    q.push(UiEvent::Remove { id });
}

fn enqueue_full_boat_path_upwards(
    q: &mut EventQueue,
    id: u32,
    mut cur: (u32, u32),
    dest: (u32, u32),
) {
    const LIFT_BRIDGE_ROW: u32 = 19;
    let (dest_row, _dest_col) = dest;

    while cur.0 > LIFT_BRIDGE_ROW {
        cur.0 -= 1;
        q.push(UiEvent::Move { id, to: cur });
    }

    if cur.0 == LIFT_BRIDGE_ROW {
        cur.0 -= 1;
        q.push(UiEvent::Move { id, to: cur });
    }

    while cur.0 > dest_row {
        cur.0 -= 1;
        q.push(UiEvent::Move { id, to: cur });
    }

    q.push(UiEvent::Remove { id });
}
