#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod fsdisp;
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
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);
        let mut terms: Vec<Option<Terminal>> = (0..5).map(|_| None).collect();
        terms[0] = Terminal::new(&cc.egui_ctx).ok();
        Self {
            terms,
            active: 0,
            sys: modules::SysPanels::new(),
            fs: fsdisp::FsDisp::new(),
            kb: keyboard::Keyboard::new("en-US"),
        }
    }
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
        self.sys.globe_phase = (now as f32 * 0.35) % std::f32::consts::TAU;
        // niente repaint a 60 fps: il globo gira a ~10 fps, il resto si aggiorna con gli eventi
        ctx.request_repaint_after(Duration::from_millis(100));

        // tasti fisici -> evidenzia sulla tastiera virtuale
        let typed: Vec<String> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(t) => Some(t.to_uppercase()),
                    _ => None,
                })
                .collect()
        });
        for t in typed {
            self.kb.flash_key(t, now);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG))
            .show(ctx, |ui| {
                let screen = ui.max_rect();
                let (w, h) = (screen.width(), screen.height());
                let (vh, vw) = (h / 100.0, w / 100.0);
                let p = ui.painter().clone();

                // ---- colonne laterali (17% ciascuna)
                let col_w = w * 0.17;
                let col_top = screen.top() + vh * 2.5;
                let col_h = vh * 96.0;
                let left = Rect::from_min_size(Pos2::new(screen.left() - vh * 0.555, col_top), Vec2::new(col_w, col_h));
                let right = Rect::from_min_size(Pos2::new(screen.right() - col_w + vh * 0.555, col_top), Vec2::new(col_w, col_h));
                self.sys.left_column(&p, left, vh);
                self.sys.right_column(&p, right, vh);

                // ---- shell principale (65% x 60.3%)
                let shell_w = w * 0.65;
                let shell_h = vh * 60.3;
                let shell = Rect::from_min_size(Pos2::new(screen.center().x - shell_w / 2.0, screen.top() + vh * 3.6), Vec2::new(shell_w, shell_h));
                theme::text(&p, shell.left() + vh * 0.9, shell.top() - vh * 0.9, Align2::LEFT_CENTER, "TERMINAL", vh * 1.02, theme::FG, "main");
                theme::text(&p, shell.right() - vh * 0.9, shell.top() - vh * 0.9, Align2::RIGHT_CENTER, "MAIN SHELL", vh * 1.02, theme::FG, "main");
                p.line_segment(
                    [Pos2::new(shell.left(), shell.top() - vh * 0.2), Pos2::new(shell.right(), shell.top() - vh * 0.2)],
                    Stroke::new(theme::px(vh, 0.092), theme::fg_a(0.3)),
                );
                shell::frame(&p, shell, vh);

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
                let active = self.active;
                if let Some(Some(t)) = self.terms.get_mut(active) {
                    t.ui(ui, term_rect, id, vh * 1.67);
                }

                // ---- filesystem e tastiera (sotto la shell)
                let bottom_top = screen.top() + vh * 67.3;
                let fs_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 0.5, bottom_top), Vec2::new(vw * 43.0, vh * 31.0));
                let kb_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 44.0, bottom_top), Vec2::new(vw * 55.5, vh * 31.0));

                if let Some(path) = self.fs.ui(ui, fs_rect, vh) {
                    if let Some(Some(t)) = self.terms.get_mut(active) {
                        t.send(&format!("cd \"{}\"\r", path.display()));
                    }
                }
                if let Some(s) = self.kb.ui(ui, kb_rect, vh, now) {
                    if let Some(Some(t)) = self.terms.get_mut(active) {
                        t.send(&s);
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 560.0])
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
