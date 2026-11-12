use eframe::egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Stroke};

// Tema "tron" di eDEX-UI
pub const FG: Color32 = Color32::from_rgb(170, 207, 209);
pub const BG: Color32 = Color32::from_rgb(5, 8, 13);
pub const GREY: Color32 = Color32::from_rgb(38, 40, 40);

/// Colore principale con trasparenza (0.0 - 1.0).
pub fn fg_a(a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(170, 207, 209, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

const ANSI16: [(u8, u8, u8); 16] = [
    (0, 0, 0), (205, 49, 49), (13, 188, 121), (229, 229, 16),
    (36, 114, 200), (188, 63, 188), (17, 168, 205), (170, 207, 209),
    (102, 102, 102), (241, 76, 76), (35, 209, 139), (245, 245, 67),
    (59, 142, 234), (214, 112, 214), (41, 184, 219), (255, 255, 255),
];

pub fn indexed(i: u8) -> Color32 {
    match i {
        0..=15 => {
            let (r, g, b) = ANSI16[i as usize];
            Color32::from_rgb(r, g, b)
        }
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
    text(p, r.left() + vh * 0.9, y, Align2::LEFT_CENTER, left, vh * 1.02, FG, "main");
    text(p, r.right() - vh * 0.9, y, Align2::RIGHT_CENTER, right, vh * 1.02, FG, "main");
    let st = Stroke::new(px(vh, 0.092), fg_a(0.3));
    let by = r.top() + vh * 1.25;
    p.line_segment([Pos2::new(r.left(), by), Pos2::new(r.right(), by)], st);
    p.line_segment([Pos2::new(r.left(), by - vh * 0.46), Pos2::new(r.left(), by)], st);
    p.line_segment([Pos2::new(r.right(), by - vh * 0.46), Pos2::new(r.right(), by)], st);
}
