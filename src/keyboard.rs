use crate::theme::{self, fg_a, px, text, BG, FG};
use eframe::egui::{self, Align2, Pos2, Rect, Sense, Stroke, Vec2};

struct Key {
    name: String,
    cmd: String,
    shift_name: String,
    shift_cmd: String,
}

pub struct Keyboard {
    rows: Vec<Vec<Key>>,
    shift: bool,
    ctrl: bool,
    pub flash: Option<(String, f64)>,
}

impl Keyboard {
    pub fn new(layout: &str) -> Self {
        let raw = match layout {
            "it-IT" => include_str!("../assets/kb_it-IT.json"),
            _ => include_str!("../assets/kb_en-US.json"),
        };
        let v: serde_json::Value = serde_json::from_str(raw).unwrap_or(serde_json::Value::Null);
        let mut rows = Vec::new();
        for k in ["row_numbers", "row_1", "row_2", "row_3", "row_space"] {
            let mut row = Vec::new();
            if let Some(arr) = v.get(k).and_then(|x| x.as_array()) {
                for key in arr {
                    let g = |f: &str| key.get(f).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    row.push(Key { name: g("name"), cmd: g("cmd"), shift_name: g("shift_name"), shift_cmd: g("shift_cmd") });
                }
            }
            rows.push(row);
        }
        Self { rows, shift: false, ctrl: false, flash: None }
    }

    /// Evidenzia il tasto fisico premuto.
    pub fn flash_key(&mut self, name: String, now: f64) {
        self.flash = Some((name, now));
    }

    /// Disegna la tastiera e ritorna il testo da inviare al terminale.
    pub fn ui(&mut self, ui: &mut egui::Ui, r: Rect, vh: f32, now: f64) -> Option<String> {
        let p = ui.painter_at(r.expand(vh));
        let mut out: Option<String> = None;
        let rows = self.rows.len().max(1);
        let row_h = r.height() / rows as f32;
        let flashing = self.flash.clone().filter(|(_, t)| now - *t < 0.15).map(|(n, _)| n);

        for (ri, row) in self.rows.iter().enumerate() {
            let y = r.top() + row_h * ri as f32;
            // pesi: primo/ultimo tasto più larghi, spazio elastico
            let weights: Vec<f32> = row
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    if k.cmd == " " {
                        6.0
                    } else if ri != 4 && (i == 0 || i == row.len() - 1) {
                        1.6
                    } else {
                        1.0
                    }
                })
                .collect();
            let sum: f32 = weights.iter().sum::<f32>().max(1.0);
            let mut x = r.left();
            for (ki, key) in row.iter().enumerate() {
                let w = r.width() * weights[ki] / sum;
                let kr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, row_h)).shrink2(Vec2::new(vh * 0.46, vh * 0.46));
                x += w;

                let id = ui.id().with(("kb", ri, ki));
                let resp = ui.interact(kr, id, Sense::click());
                let active = resp.is_pointer_button_down_on()
                    || (key.name == "SHIFT" && self.shift)
                    || (key.name == "CTRL" && self.ctrl)
                    || flashing.as_deref().map(|f| f.eq_ignore_ascii_case(&key.name)).unwrap_or(false);

                p.rect_stroke(kr, 0.0, Stroke::new(px(vh, 0.18), fg_a(if active { 1.0 } else { 0.0 })));
                if active {
                    p.rect_filled(kr, 0.0, FG);
                }
                let col = if active { BG } else { FG };
                if let Some(dir) = key.name.strip_prefix("ESCAPED|-- ICON: ARROW_") {
                    arrow(&p, kr, dir, col, vh);
                    if resp.clicked() {
                        out = Some(
                            match dir {
                                "UP" => "\x1b[A",
                                "DOWN" => "\x1b[B",
                                "RIGHT" => "\x1b[C",
                                _ => "\x1b[D",
                            }
                            .to_string(),
                        );
                    }
                    continue;
                }
                let main = if self.shift && !key.shift_name.is_empty() { &key.shift_name } else { &key.name };
                let long = main.chars().count() > 1;
                text(&p, kr.center().x, kr.center().y, Align2::CENTER_CENTER, main, vh * if long { 1.85 } else { 2.4 }, col, "main");
                if !self.shift && !key.shift_name.is_empty() && !long {
                    text(&p, kr.right() - vh * 0.7, kr.top() + vh * 0.9, Align2::RIGHT_CENTER, &key.shift_name, vh * 1.3, fg_a(if active { 0.8 } else { 0.5 }), "main");
                }

                if resp.clicked() {
                    match key.name.as_str() {
                        "SHIFT" => self.shift = !self.shift,
                        "CTRL" => self.ctrl = !self.ctrl,
                        "ESC" => out = Some("\x1b".into()),
                        "TAB" => out = Some("\t".into()),
                        "BACK" | "BACKSPACE" => out = Some("\x7f".into()),
                        "ENTER" | "RETURN" => out = Some("\r".into()),
                        _ => {
                            let mut c = if self.shift && !key.shift_cmd.is_empty() { key.shift_cmd.clone() } else { key.cmd.clone() };
                            if c.starts_with("ESCAPED|") || c.starts_with("~~~") {
                                c.clear();
                            }
                            if self.ctrl && c.len() == 1 {
                                let b = c.as_bytes()[0].to_ascii_lowercase();
                                if b.is_ascii_lowercase() {
                                    c = ((b - b'a' + 1) as char).to_string();
                                }
                                self.ctrl = false;
                            }
                            if !c.is_empty() {
                                out = Some(c);
                                self.shift = false;
                            }
                        }
                    }
                }
            }
        }
        let _ = theme::FG;
        out
    }
}

fn arrow(p: &egui::Painter, r: Rect, dir: &str, col: egui::Color32, vh: f32) {
    let c = r.center();
    let s = vh * 1.1;
    let pts = match dir {
        "UP" => [c + Vec2::new(0.0, -s), c + Vec2::new(s, s * 0.7), c + Vec2::new(-s, s * 0.7)],
        "DOWN" => [c + Vec2::new(0.0, s), c + Vec2::new(s, -s * 0.7), c + Vec2::new(-s, -s * 0.7)],
        "LEFT" => [c + Vec2::new(-s, 0.0), c + Vec2::new(s * 0.7, -s), c + Vec2::new(s * 0.7, s)],
        _ => [c + Vec2::new(s, 0.0), c + Vec2::new(-s * 0.7, -s), c + Vec2::new(-s * 0.7, s)],
    };
    p.add(egui::Shape::convex_polygon(pts.to_vec(), col, Stroke::NONE));
}
