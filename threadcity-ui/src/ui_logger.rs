use gtk::TextBuffer;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::event_queue::{EventQueue, UiEvent, EntityKind};
use crate::ui::drawing::RIVER_COL;

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
        // reglas simples de parseo para crear y mover entidades
        // los formatos de runner usan coords debug como Coord { x: A, y: B }

        // carro creado
        if let Some(id) = extract_id(line, "Carro-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn { id, kind: EntityKind::Car, pos: o });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        // ambulancia creada
        if let Some(id) = extract_id(line, "Ambulancia-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn { id, kind: EntityKind::Ambulance, pos: o });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        // camiones creados
        if let Some(id) = extract_id(line, "CargoTruck-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn { id, kind: EntityKind::Truck, pos: o });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        // barcos creados
        if let Some(id) = extract_id(line, "Barco-") {
            if let Some((o, d)) = extract_origin_dest(line) {
                let mut q = self.events.borrow_mut();
                q.push(UiEvent::Spawn { id, kind: EntityKind::Boat, pos: o });
                enqueue_path(&mut *q, id, o, d);
                return;
            }
        }

        // llego a destino
        if line.contains("✅ Llegó a destino") || line.contains("⛵ Cruzó el puente, pos:") {
            if let Some(id) = extract_bracket_id(line) {
                let mut q = self.events.borrow_mut();
                // si es cruzo el puente con pos entonces mover a esa pos
                if let Some(pos) = extract_single_coord(line) {
                    q.push(UiEvent::Move { id, to: pos });
                    return;
                }
                // si es llegada a destino remover
                q.push(UiEvent::Remove { id });
                return;
            }
        }

        // fallback registrar como log simple
        self.events.borrow_mut().push(UiEvent::Log(line.to_string()));
    }
}

// helpers de parseo

fn extract_id(s: &str, prefix: &str) -> Option<u32> {
    if let Some(i) = s.find(prefix) {
        let rest = &s[i + prefix.len()..];
        let mut num = String::new();
        for ch in rest.chars() {
            if ch.is_ascii_digit() { num.push(ch); } else { break; }
        }
        return num.parse::<u32>().ok();
    }
    None
}

fn extract_bracket_id(s: &str) -> Option<u32> {
    // busca patron como [123]
    if let Some(open) = s.find('[') {
        if let Some(close) = s[open+1..].find(']') {
            let inside = &s[open+1..open+1+close];
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
    // espera dos coords en linea usando formato debug Coord { x: A, y: B }
    let mut out = Vec::new();
    let mut rest = s;
    while let Some((x, y)) = extract_coord_from(rest) {
        out.push((x, y));
        if let Some(pos) = rest.find("y:") {
            rest = &rest[pos + 2..];
        } else {
            break;
        }
        if out.len() == 2 { break; }
    }
    if out.len() == 2 { Some((out[0], out[1])) } else { None }
}

fn extract_coord_from(s: &str) -> Option<(u32, u32)> {
    // busca x: A, y: B
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

    // ¿están en lados distintos del río?
    let needs_cross = (cur.1 < RIVER_COL && dest_col > RIVER_COL)
        || (cur.1 > RIVER_COL && dest_col < RIVER_COL);

    if needs_cross {
        // filas de puentes reales según CityLayout::default()
        let bridge_rows = [1u32, 2, 3];

        // elegimos la fila de puente más cercana a la fila actual
        let mut cross_row = bridge_rows[0];
        let mut best = cur.0.abs_diff(bridge_rows[0]);
        for &br in &bridge_rows[1..] {
            let d = cur.0.abs_diff(br);
            if d < best {
                best = d;
                cross_row = br;
            }
        }

        // 1) subir/bajar hasta la fila del puente, sin cruzar el río aún
        while cur.0 != cross_row {
            if cur.0 < cross_row {
                cur.0 += 1;
            } else {
                cur.0 -= 1;
            }
            q.push(UiEvent::Move { id, to: cur });
        }

        // 2) cruzar el río en esa fila, saltando la columna del río
        while cur.1 != dest_col {
            if cur.1 < dest_col {
                if cur.1 + 1 == RIVER_COL {
                    // saltamos directamente al otro lado
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

        // 3) una vez del otro lado, ajustar la fila hasta el destino
        while cur.0 != dest_row {
            if cur.0 < dest_row {
                cur.0 += 1;
            } else {
                cur.0 -= 1;
            }
            q.push(UiEvent::Move { id, to: cur });
        }
    } else {
        // caso simple: no hay cruce de río, comportamiento anterior
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
