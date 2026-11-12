#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod panels;
mod terminal;
mod theme;

use eframe::egui::{self, Color32, RichText, Stroke};
use panels::{FsBrowser, SysPanels};
use std::time::Duration;
use terminal::Terminal;

struct App {
    term: Result<Terminal, String>,
    sys: SysPanels,
    fs: FsBrowser,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_style(&cc.egui_ctx);
        let term = Terminal::new(&cc.egui_ctx).map_err(|e| e.to_string());
        Self { term, sys: SysPanels::new(), fs: FsBrowser::new() }
    }
}

fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(theme::FG);
    v.panel_fill = theme::BG;
    v.window_fill = theme::BG;
    v.extreme_bg_color = theme::BG;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, theme::GREY);
    v.widgets.hovered.bg_fill = theme::GREY;
    v.widgets.hovered.weak_bg_fill = theme::GREY;
    v.widgets.active.bg_fill = theme::GREY;
    v.selection.bg_fill = theme::FG.gamma_multiply(0.3);
    v.selection.stroke = Stroke::new(1.0, theme::FG);
    ctx.set_visuals(v);
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from_rgb(5, 8, 13).to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh();
        // niente repaint continuo: solo 1 volta al secondo (o quando arriva output dal PTY)
        ctx.request_repaint_after(Duration::from_millis(1000));

        let side = egui::Frame::none().inner_margin(egui::Margin::same(6.0)).fill(theme::BG);

        egui::TopBottomPanel::top("header").frame(side).show(ctx, |ui| panels::header(ui));

        egui::SidePanel::left("left")
            .frame(side)
            .exact_width(250.0)
            .resizable(false)
            .show(ctx, |ui| self.sys.left(ui));

        let mut cd_to = None;
        egui::SidePanel::right("right")
            .frame(side)
            .exact_width(270.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.sys.network(ui);
                cd_to = self.fs.ui(ui);
            });

        egui::CentralPanel::default().frame(side).show(ctx, |ui| match &mut self.term {
            Ok(t) => {
                if let Some(p) = cd_to.take() {
                    t.send(&format!("cd \"{}\"\r", p.display()));
                }
                t.ui(ui);
            }
            Err(e) => {
                ui.label(RichText::new(format!("Impossibile avviare il terminale: {e}")).color(Color32::LIGHT_RED));
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([900.0, 500.0])
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
