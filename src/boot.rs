use crate::audio::Snd;
use crate::theme::{self, fg_a, px, BG, FG};
use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

/// Durata dell'animazione di comparsa dell'interfaccia dopo il titolo (secondi).
pub const INTRO_LEN: f64 = 6.7;

/// Sequenza di avvio dell'originale: boot log, schermata del titolo con glitch,
/// poi comparsa dell'interfaccia. Tutti i tempi seguono quelli di eDEX-UI.
pub struct Timeline {
    events: Vec<(f64, Snd)>,
    chunks: Vec<(f64, String)>,
    pub title_start: f64,
    pub ui_start: f64,
    next: usize,
}

impl Timeline {
    pub fn new() -> Self {
        let raw = include_str!("../assets/boot_log.txt");
        let log: Vec<&str> = raw.split('\n').collect();
        let n = log.len();
        let kernel = format!(
            "eDEX-UI Kernel version 2.2.8 boot at {}; root:xnu-1699.22.73~1/RELEASE_X86_64",
            chrono::Local::now().format("%a %b %d %Y %H:%M:%S GMT%z")
        );

        let mut events: Vec<(f64, Snd)> = Vec::new();
        let mut chunks: Vec<(f64, String)> = Vec::new();
        let (mut t, mut i) = (0.0f64, 0usize);
        while i < n {
            events.push((t, if log[i] == "Boot Complete" { Snd::Granted } else { Snd::Stdout }));
            chunks.push((t, format!("{}\n", log[i])));
            i += 1;
            if i == 2 {
                chunks.push((t, kernel.clone()));
            }
            t += match i {
                2 | 4 => 0.5,
                5..=24 => 0.03,
                25 => 0.4,
                42 => 0.3,
                43..=81 => 0.025,
                83 => 0.025,
                x if x + 2 >= n && x < n => 0.3,
                _ => (1.0 - i as f64 / 1000.0).powi(3) * 0.025,
            };
        }
        let title_start = t + 0.3;
        let ui_start = title_start + 2.6;

        events.push((title_start, Snd::Theme));
        events.push((ui_start + 0.01, Snd::Expand));
        events.push((ui_start + 1.48, Snd::Keyboard));
        events.push((ui_start + 3.08, Snd::Scan));
        for k in 0..6 {
            events.push((ui_start + 3.08 + 0.5 * (k as f64 + 1.0), Snd::Panels));
        }
        events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Self { events, chunks, title_start, ui_start, next: 0 }
    }

    /// Suoni da riprodurre fino al tempo `t`.
    pub fn poll(&mut self, t: f64) -> Vec<Snd> {
        let mut out = Vec::new();
        while self.next < self.events.len() && self.events[self.next].0 <= t {
            out.push(self.events[self.next].1);
            self.next += 1;
        }
        out
    }

    pub fn draw_boot_log(&self, p: &Painter, screen: Rect, vh: f32, t: f64) {
        let mut s = String::new();
        for (ct, c) in &self.chunks {
            if *ct <= t {
                s.push_str(c);
            }
        }
        let g = p.layout(s, theme::font(vh * 1.4, "mono"), Color32::PLACEHOLDER, screen.width());
        let pos = Pos2::new(screen.left(), screen.bottom() - g.size().y);
        p.galley(pos, g, FG);
    }
}

/// Schermata del titolo "eDEX-UI": comparsa, riempimento, contorno, glitch. `t` parte da 0 all'inizio del titolo.
pub fn draw_title(p: &Painter, screen: Rect, vh: f32, t: f64) {
    if t < 0.4 {
        return;
    }
    let fade = (((t - 0.4) / 0.3).clamp(0.0, 1.0)) as f32;
    let g = p.layout_no_wrap("eDEX-UI".into(), theme::font(vh * 10.0, "main"), Color32::PLACEHOLDER);
    let (tw, th) = (g.size().x, g.size().y);
    let (pl, pr, pt) = (vh * 1.5, vh * 2.0, vh * 2.0);
    let b = Rect::from_center_size(screen.center(), Vec2::new(tw + pl + pr, th + pt));
    let pos = Pos2::new(b.left() + pl, b.top() + pt);
    let bw = px(vh, 0.46);
    let line = Stroke::new(bw, fg_a(fade));

    if t < 0.7 {
        p.galley(pos, g, fg_a(fade));
        p.line_segment([b.left_bottom(), b.right_bottom()], line);
    } else if t < 1.0 {
        p.rect_filled(b, 0.0, FG);
        p.galley(pos, g, FG);
    } else if t < 1.1 {
        p.rect_stroke(b, 0.0, Stroke::new(bw, FG));
    } else if t < 1.6 {
        // glitch: due copie del testo (parte alta e bassa) che oscillano ogni 50 ms
        let ph = (t * 20.0) % 2.0;
        let f = (if ph < 1.0 { ph } else { 2.0 - ph }) as f32;
        let top = Rect::from_min_max(Pos2::new(b.left() - tw, pos.y), Pos2::new(b.right() + tw, pos.y + th * 0.4));
        let bot = Rect::from_min_max(Pos2::new(b.left() - tw, pos.y + th * 0.4), Pos2::new(b.right() + tw, pos.y + th));
        let dx_top = tw * (-0.01 + (-0.05 + 0.01) * f);
        let dx_bot = tw * (0.01 + 0.02 * f);
        p.with_clip_rect(top).galley(Pos2::new(pos.x + dx_top, pos.y), g.clone(), fg_a(0.8));
        p.with_clip_rect(bot).galley(Pos2::new(pos.x + dx_bot, pos.y), g, fg_a(0.9));
    } else {
        p.galley(pos, g, FG);
        p.rect_stroke(b, 0.0, Stroke::new(bw, FG));
    }
    let _ = BG;
}
