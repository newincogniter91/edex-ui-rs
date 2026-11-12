use eframe::egui::Color32;

// Tema "tron" originale di eDEX-UI
pub const FG: Color32 = Color32::from_rgb(170, 207, 209);
pub const BG: Color32 = Color32::from_rgb(5, 8, 13);
pub const GREY: Color32 = Color32::from_rgb(38, 40, 40);
pub const DIM: Color32 = Color32::from_rgba_premultiplied(85, 103, 104, 255);

const ANSI16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 49, 49),
    (13, 188, 121),
    (229, 229, 16),
    (36, 114, 200),
    (188, 63, 188),
    (17, 168, 205),
    (170, 207, 209),
    (102, 102, 102),
    (241, 76, 76),
    (35, 209, 139),
    (245, 245, 67),
    (59, 142, 234),
    (214, 112, 214),
    (41, 184, 219),
    (255, 255, 255),
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
