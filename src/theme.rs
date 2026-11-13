use eframe::egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke};
use std::sync::RwLock;

/// Palette corrente (cambia con il tema scelto nelle impostazioni).
#[derive(Clone, Copy)]
pub struct Palette {
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub grey: [u8; 3],
    pub term_fg: [u8; 3],
    pub term_bg: [u8; 3],
    pub cursor: [u8; 3],
    pub ansi: [[u8; 3]; 16],
}

// Colori ANSI di xterm.js (quelli usati dall'originale) e tema "tron" di default.
const TANGO: [[u8; 3]; 16] = [
    [0x2e, 0x34, 0x36], [0xcc, 0x00, 0x00], [0x4e, 0x9a, 0x06], [0xc4, 0xa0, 0x00],
    [0x34, 0x65, 0xa4], [0x75, 0x50, 0x7b], [0x06, 0x98, 0x9a], [0xd3, 0xd7, 0xcf],
    [0x55, 0x57, 0x53], [0xef, 0x29, 0x29], [0x8a, 0xe2, 0x34], [0xfc, 0xe9, 0x4f],
    [0x72, 0x9f, 0xcf], [0xad, 0x7f, 0xa8], [0x34, 0xe2, 0xe2], [0xee, 0xee, 0xec],
];

pub const DEFAULT: Palette = Palette {
    fg: [170, 207, 209],
    bg: [5, 8, 13],
    grey: [38, 40, 40],
    term_fg: [170, 207, 209],
    term_bg: [5, 8, 13],
    cursor: [170, 207, 209],
    ansi: TANGO,
};

static PAL: RwLock<Palette> = RwLock::new(DEFAULT);

pub fn set_palette(p: Palette) {
    if let Ok(mut g) = PAL.write() {
        *g = p;
    }
}

fn pal() -> Palette {
    PAL.read().map(|g| *g).unwrap_or(DEFAULT)
}

fn c(v: [u8; 3]) -> Color32 {
    Color32::from_rgb(v[0], v[1], v[2])
}

pub fn fg() -> Color32 {
    c(pal().fg)
}
pub fn bg() -> Color32 {
    c(pal().bg)
}
pub fn grey() -> Color32 {
    c(pal().grey)
}
pub fn term_fg() -> Color32 {
    c(pal().term_fg)
}
pub fn term_bg() -> Color32 {
    c(pal().term_bg)
}
pub fn cursor() -> Color32 {
    c(pal().cursor)
}

/// Colore principale con trasparenza (0.0 - 1.0).
pub fn fg_a(a: f32) -> Color32 {
    let f = pal().fg;
    Color32::from_rgba_unmultiplied(f[0], f[1], f[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Colore di sfondo con trasparenza (per le dissolvenze).
pub fn bg_a(a: f32) -> Color32 {
    let b = pal().bg;
    Color32::from_rgba_unmultiplied(b[0], b[1], b[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn indexed(i: u8) -> Color32 {
    match i {
        0..=15 => c(pal().ansi[i as usize]),
        16..=231 => {
            let n = i - 16;
            let lv = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Color32::from_rgb(lv(n / 36), lv((n / 6) % 6), lv(n % 6))
        }
        _ => {
            let g = 8 + (i - 232) * 10;
            Color32::from_rgb(g, g, g)
        }
    }
}

/// Famiglie di font: "main" (United Sans Medium), "light" (United Sans Light), "mono" (Fira Mono).
pub fn font(size: f32, fam: &str) -> FontId {
    let f = match fam {
        "main" => FontFamily::Name("main".into()),
        "light" => FontFamily::Name("light".into()),
        _ => FontFamily::Monospace,
    };
    FontId::new(size, f)
}

#[allow(clippy::too_many_arguments)]
pub fn text(p: &Painter, x: f32, y: f32, a: Align2, s: impl ToString, size: f32, col: Color32, fam: &str) {
    p.text(Pos2::new(x, y), a, s.to_string(), font(size, fam), col);
}

pub fn px(vh: f32, v: f32) -> f32 {
    (vh * v).max(1.0)
}

/// Bordo superiore con i due "tick" agli angoli, tipico dei moduli eDEX.
pub fn module_frame(p: &Painter, r: Rect, vh: f32) {
    let st = Stroke::new(px(vh, 0.092), fg_a(0.3));
    p.line_segment([r.left_top(), r.right_top()], st);
    let t = vh * 0.83;
    p.line_segment([Pos2::new(r.left(), r.top() - t * 0.25), Pos2::new(r.left(), r.top() + t * 0.75)], st);
    p.line_segment([Pos2::new(r.right(), r.top() - t * 0.25), Pos2::new(r.right(), r.top() + t * 0.75)], st);
}

/// Titolo di colonna/pannello: testo sinistro e destro con linea sotto e tick.
pub fn title_bar(p: &Painter, r: Rect, left: &str, right: &str, vh: f32) {
    let y = r.top() + vh * 0.5;
    text(p, r.left() + vh * 0.9, y, Align2::LEFT_CENTER, left, vh * 1.02, fg(), "main");
    text(p, r.right() - vh * 0.9, y, Align2::RIGHT_CENTER, right, vh * 1.02, fg(), "main");
    let st = Stroke::new(px(vh, 0.092), fg_a(0.3));
    let by = r.top() + vh * 1.25;
    p.line_segment([Pos2::new(r.left(), by), Pos2::new(r.right(), by)], st);
    p.line_segment([Pos2::new(r.left(), by - vh * 0.46), Pos2::new(r.left(), by)], st);
    p.line_segment([Pos2::new(r.right(), by - vh * 0.46), Pos2::new(r.right(), by)], st);
}

#[cfg(test)]
pub trait IntoColor {
    fn into_color(self) -> Color32;
}
#[cfg(test)]
impl IntoColor for [u8; 3] {
    fn into_color(self) -> Color32 {
        Color32::from_rgb(self[0], self[1], self[2])
    }
}
