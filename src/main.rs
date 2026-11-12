#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod audio;
mod boot;
mod fsdisp;
mod hw;
mod icons;
mod keyboard;
mod modules;
mod shell;
mod terminal;
mod theme;

use eframe::egui::{self, Align2, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, Vec2};
use std::time::Duration;
use terminal::Terminal;

struct App {
    terms: Vec<Option<Terminal>>,
    active: usize,
    sys: modules::SysPanels,
    fs: fsdisp::FsDisp,
    kb: keyboard::Keyboard,
    audio: audio::Audio,
    tl: boot::Timeline,
    t0: Option<f64>,
    fs_started: bool,
    last_stdout: f64,
    user: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);
        let mut terms: Vec<Option<Terminal>> = (0..5).map(|_| None).collect();
        terms[0] = Terminal::new(&cc.egui_ctx).ok();
        let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
        Self {
            terms,
            active: 0,
            sys: modules::SysPanels::new(),
            fs: fsdisp::FsDisp::new(),
            kb: keyboard::Keyboard::new("en-US"),
            audio: audio::Audio::new(),
            tl: boot::Timeline::new(),
            t0: None,
            fs_started: false,
            last_stdout: 0.0,
            user,
        }
    }
}

fn clamp01(x: f64) -> f32 {
    x.clamp(0.0, 1.0) as f32
}

/// Fira Mono è incorporato (licenza OFL). United Sans (House Industries) non è ridistribuibile:
/// se i file `united_sans_medium.ttf` / `united_sans_light.ttf` sono accanto all'exe
/// (o in `fonts/`), vengono usati; altrimenti si usa il font proporzionale di egui.
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("fira".into(), FontData::from_static(include_bytes!("../assets/fonts/fira_mono.ttf")));
    if let Some(m) = fonts.families.get_mut(&FontFamily::Monospace) {
        m.insert(0, "fira".into());
    }

    let mut dirs = vec![std::path::PathBuf::from("."), std::path::PathBuf::from("fonts")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            dirs.push(d.to_path_buf());
            dirs.push(d.join("fonts"));
        }
    }
    let base: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (key, file, fam) in [("us_medium", "united_sans_medium.ttf", "main"), ("us_light", "united_sans_light.ttf", "light")] {
        let mut list: Vec<String> = Vec::new();
        for d in &dirs {
            if let Ok(bytes) = std::fs::read(d.join(file)) {
                fonts.font_data.insert(key.into(), FontData::from_owned(bytes));
                list.push(key.into());
                break;
            }
        }
        list.extend(base.clone());
        fonts.families.insert(FontFamily::Name(fam.into()), list);
    }
    ctx.set_fonts(fonts);
}

fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(theme::FG);
    v.panel_fill = theme::BG;
    v.window_fill = theme::BG;
    v.extreme_bg_color = theme::BG;
    v.selection.bg_fill = theme::fg_a(0.3);
    v.selection.stroke = Stroke::new(1.0_f32, theme::FG);
    v.widgets.hovered.bg_fill = theme::GREY;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.scroll = egui::style::ScrollStyle::solid();
        s.spacing.scroll.bar_width = 8.0;
    });
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(5, 8, 13).to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh();
        let now = ctx.input(|i| i.time);
        let t0 = *self.t0.get_or_insert(now);
        let t = now - t0;

        for s in self.tl.poll(t) {
            self.audio.play(s);
        }
        let ui_t = t - self.tl.ui_start; // < 0 durante boot e titolo
        let booting = ui_t < 0.0;
        let intro_done = ui_t >= boot::INTRO_LEN;
        let term_visible = ui_t >= 3.18;

        self.sys.globe_phase = (now as f32 * 0.35) % std::f32::consts::TAU;
        // durante l'avvio si ridisegna fluido; poi ~10 fps (il globo gira piano)
        let wait = if intro_done { 100 } else { 16 };
        ctx.request_repaint_after(Duration::from_millis(wait));

        // suoni dei tasti fisici (come nell'originale: keydown -> stdin, Invio rilasciato -> granted)
        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        if term_visible {
            for e in &events {
                match e {
                    egui::Event::Key { pressed: true, .. } => self.audio.play(audio::Snd::Stdin),
                    egui::Event::Key { key: egui::Key::Enter, pressed: false, .. } => self.audio.play(audio::Snd::Granted),
                    _ => {}
                }
            }
            // output del terminale -> stdout, al massimo ogni 30 ms
            let mut got = false;
            for tm in self.terms.iter().flatten() {
                got |= tm.take_output();
            }
            if got && now - self.last_stdout > 0.03 {
                self.audio.play(audio::Snd::Stdout);
                self.last_stdout = now;
            }
        }

        // tasti fisici -> evidenzia sulla tastiera virtuale
        for e in &events {
            if let egui::Event::Text(tx) = e {
                self.kb.flash_key(tx.to_uppercase(), now);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let (w, h) = (screen.width(), screen.height());
                let (vh, vw) = (h / 100.0, w / 100.0);
                let p = ui.painter().clone();

                // ---- boot log e titolo
                if booting {
                    if t < self.tl.title_start {
                        self.tl.draw_boot_log(&p, screen, vh, t);
                    } else {
                        boot::draw_title(&p, screen, vh, t - self.tl.title_start);
                    }
                    return;
                }

                // ---- opacità/tempi dell'animazione di comparsa (stessi tempi dell'originale)
                let it = ui_t;
                let shell_wf = clamp01((it - 0.01) / 0.5);
                let shell_hf = clamp01((it - 0.51) / 0.5).max(0.004);
                let title_a = clamp01((it - 1.01) / 0.5);
                let greet_a = clamp01((it - 1.48) / 0.5).min(1.0 - clamp01((it - 2.58) / 0.5));
                let fsk_a = clamp01((it - 1.48) / 0.5);
                let mut fade = modules::Fade::full();
                fade.title = clamp01((it - 3.08) / 0.5);
                for i in 0..6 {
                    fade.left[i] = clamp01((it - (3.08 + 0.5 * (i as f64 + 1.0))) / 0.5);
                }
                for i in 0..3 {
                    fade.right[i] = clamp01((it - (3.08 + 0.5 * (i as f64 + 1.0))) / 0.5);
                }
                if !self.fs_started && it >= 1.48 {
                    self.fs_started = true;
                    self.fs.restart(now);
                }

                // ---- colonne laterali (17% ciascuna), accorciate per non toccare filesystem e tastiera
                let col_w = w * 0.17;
                let col_top = screen.top() + vh * 2.5;
                let col_h = vh * 63.7;
                let left = Rect::from_min_size(Pos2::new(screen.left() - vh * 0.555, col_top), Vec2::new(col_w, col_h));
                let right = Rect::from_min_size(Pos2::new(screen.right() - col_w + vh * 0.555, col_top), Vec2::new(col_w, col_h));
                self.sys.left_column(&p, left, vh, &fade);
                self.sys.right_column(&p, right, vh, &fade);

                // ---- shell principale (65% x 60.3%)
                let shell_w = w * 0.65;
                let shell_h = vh * 60.3;
                let shell = Rect::from_min_size(Pos2::new(screen.center().x - shell_w / 2.0, screen.top() + vh * 3.6), Vec2::new(shell_w, shell_h));
                let anim = Rect::from_center_size(shell.center(), Vec2::new(shell_w * shell_wf, shell_h * shell_hf));
                if shell_wf > 0.0 {
                    shell::frame(&p, anim, vh);
                }
                if title_a > 0.0 {
                    theme::text(&p, shell.left() + vh * 0.9, shell.top() - vh * 0.9, Align2::LEFT_CENTER, "TERMINAL", vh * 1.02, theme::fg_a(title_a), "main");
                    theme::text(&p, shell.right() - vh * 0.9, shell.top() - vh * 0.9, Align2::RIGHT_CENTER, "MAIN SHELL", vh * 1.02, theme::fg_a(title_a), "main");
                    p.line_segment(
                        [Pos2::new(shell.left(), shell.top() - vh * 0.2), Pos2::new(shell.right(), shell.top() - vh * 0.2)],
                        Stroke::new(theme::px(vh, 0.092), theme::fg_a(0.3 * title_a)),
                    );
                }
                if greet_a > 0.0 {
                    let name = if self.user.is_empty() { String::new() } else { format!(", {}", self.user) };
                    theme::text(&p, shell.center().x, shell.center().y, Align2::CENTER_CENTER, format!("Welcome back{name}"), vh * 3.9, theme::fg_a(greet_a), "main");
                }

                let active = self.active;
                if term_visible {
                    let tab_h = vh * 3.3;
                    let tabs_rect = Rect::from_min_size(shell.min, Vec2::new(shell.width(), tab_h));
                    let labels: Vec<String> = (0..5)
                        .map(|i| {
                            if i == 0 {
                                "MAIN SHELL".to_string()
                            } else if self.terms[i].is_some() {
                                format!("SHELL #{}", i + 1)
                            } else {
                                "EMPTY".to_string()
                            }
                        })
                        .collect();
                    if let Some(i) = shell::tabs(ui, tabs_rect, &labels, self.active, vh) {
                        self.audio.play(audio::Snd::Folder);
                        if self.terms[i].is_none() {
                            self.terms[i] = Terminal::new(ctx).ok();
                        }
                        if self.terms[i].is_some() {
                            self.active = i;
                        }
                    }

                    let term_rect = Rect::from_min_max(
                        Pos2::new(shell.left() + vh * 0.74, shell.top() + tab_h + vh * 0.6),
                        Pos2::new(shell.right() - vh * 0.74, shell.bottom() - vh * 0.74),
                    );
                    let id = ui.id().with(("term", self.active));
                    if let Some(Some(tm)) = self.terms.get_mut(active) {
                        tm.ui(ui, term_rect, id, vh * 1.67);
                    }
                }

                // ---- filesystem e tastiera (sotto la shell)
                if it >= 1.48 {
                    let bottom_top = screen.top() + vh * 67.3;
                    let fs_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 0.5, bottom_top), Vec2::new(vw * 43.0, vh * 31.0));
                    let kb_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 44.0, bottom_top), Vec2::new(vw * 55.5, vh * 31.0));

                    let nav = self.fs.ui(ui, fs_rect, vh, now);
                    if fsk_a < 1.0 {
                        p.rect_filled(fs_rect.expand(vh), 0.0, Color32::from_rgba_unmultiplied(5, 8, 13, ((1.0 - fsk_a) * 255.0) as u8));
                    }
                    let key = self.kb.ui(ui, kb_rect, vh, now);
                    self.kb.veil(&p, kb_rect, vh, (it - 1.58).max(0.0), fsk_a);

                    for s in self.fs.sounds.drain(..) {
                        self.audio.play(s);
                    }
                    let ksounds: Vec<audio::Snd> = self.kb.sounds.drain(..).collect();
                    if term_visible {
                        for s in ksounds {
                            self.audio.play(s);
                        }
                        if let Some(path) = nav {
                            if let Some(Some(tm)) = self.terms.get_mut(active) {
                                tm.send(&format!("cd \"{}\"\r", path.display()));
                            }
                        }
                        if let Some(sq) = key {
                            if let Some(Some(tm)) = self.terms.get_mut(active) {
                                tm.send(&sq);
                            }
                        }
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_icon(egui::IconData { rgba: include_bytes!("../assets/app_icon.rgba").to_vec(), width: 128, height: 128 })
            .with_title("eDEX-UI RS"),
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("eDEX-UI RS", options, Box::new(|cc| Box::new(App::new(cc))))
}
