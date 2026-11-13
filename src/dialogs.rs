use crate::data::LAYOUTS;
use crate::modules::{bytes, short, ProcRow};
use crate::settings::{theme_names, Settings};
use crate::theme;
use eframe::egui::{self, Align2, Key, RichText, Stroke, Vec2};
use std::path::PathBuf;

fn frame() -> egui::Frame {
    egui::Frame::none()
        .fill(theme::bg())
        .stroke(Stroke::new(1.5_f32, theme::fg()))
        .inner_margin(egui::Margin::same(14.0))
}

fn title(s: &str) -> RichText {
    RichText::new(s).font(theme::font(18.0, "main")).color(theme::fg())
}

fn lbl(ui: &mut egui::Ui, s: &str) {
    ui.label(RichText::new(s).font(theme::font(14.0, "main")).color(theme::fg()));
}

pub fn settings_window(ctx: &egui::Context, open: &mut bool, st: &mut Settings) {
    if !*open {
        return;
    }
    egui::Window::new(title("SETTINGS"))
        .open(open)
        .frame(frame())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Grid::new("settings_grid").num_columns(2).spacing([18.0, 8.0]).show(ui, |ui| {
                lbl(ui, "Theme");
                egui::ComboBox::from_id_source("cb_theme").selected_text(st.theme.clone()).show_ui(ui, |ui| {
                    for n in theme_names() {
                        ui.selectable_value(&mut st.theme, n.to_string(), n);
                    }
                });
                ui.end_row();

                lbl(ui, "Keyboard layout");
                egui::ComboBox::from_id_source("cb_kb").selected_text(st.keyboard.clone()).show_ui(ui, |ui| {
                    for (n, _) in LAYOUTS {
                        ui.selectable_value(&mut st.keyboard, n.to_string(), *n);
                    }
                });
                ui.end_row();

                lbl(ui, "Terminal font size");
                ui.add(egui::DragValue::new(&mut st.term_font_size).clamp_range(8.0..=40.0));
                ui.end_row();

                lbl(ui, "Audio");
                ui.checkbox(&mut st.audio, "");
                ui.end_row();
                lbl(ui, "Audio volume");
                ui.add(egui::Slider::new(&mut st.audio_volume, 0.0..=1.0));
                ui.end_row();
                lbl(ui, "Disable feedback audio");
                ui.checkbox(&mut st.disable_feedback_audio, "");
                ui.end_row();

                lbl(ui, "Clock hours");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut st.clock_hours, 24u8, "24");
                    ui.selectable_value(&mut st.clock_hours, 12u8, "12");
                });
                ui.end_row();

                lbl(ui, "Ping address");
                ui.add(egui::TextEdit::singleline(&mut st.ping_addr).desired_width(180.0));
                ui.end_row();
                lbl(ui, "Geolocate IP (ip-api.com)");
                ui.checkbox(&mut st.geoip, "");
                ui.end_row();

                lbl(ui, "Hide dotfiles");
                ui.checkbox(&mut st.hide_dotfiles, "");
                ui.end_row();
                lbl(ui, "File list view");
                ui.checkbox(&mut st.fs_list_view, "");
                ui.end_row();
                lbl(ui, "No cursor");
                ui.checkbox(&mut st.nocursor, "");
                ui.end_row();

                lbl(ui, "Shell (new tabs)");
                ui.add(egui::TextEdit::singleline(&mut st.shell).desired_width(180.0));
                ui.end_row();
                lbl(ui, "Shell arguments");
                ui.add(egui::TextEdit::singleline(&mut st.shell_args).desired_width(180.0));
                ui.end_row();
                lbl(ui, "Start directory");
                ui.add(egui::TextEdit::singleline(&mut st.cwd).desired_width(180.0));
                ui.end_row();

                lbl(ui, "Skip intro (next start)");
                ui.checkbox(&mut st.nointro, "");
                ui.end_row();
                lbl(ui, "Windowed (next start)");
                ui.checkbox(&mut st.allow_windowed, "");
                ui.end_row();
            });
            ui.add_space(8.0);
            ui.label(RichText::new(format!("File: {}", crate::settings::config_dir().join("settings.json").display())).size(11.0).color(theme::fg_a(0.6)));
        });
}

pub fn shortcuts_window(ctx: &egui::Context, open: &mut bool) {
    if !*open {
        return;
    }
    const LIST: &[(&str, &str)] = &[
        ("Ctrl+Shift+C", "Copy selection"),
        ("Ctrl+Shift+V", "Paste"),
        ("Right click", "Paste"),
        ("Ctrl+Tab", "Next tab"),
        ("Ctrl+Shift+Tab", "Previous tab"),
        ("Ctrl+1 … Ctrl+5", "Switch to tab"),
        ("Ctrl+Shift+S", "Settings"),
        ("Ctrl+Shift+K", "Shortcuts"),
        ("Ctrl+Shift+F", "Fuzzy finder"),
        ("Ctrl+Shift+L", "File list view"),
        ("Ctrl+Shift+H", "Hide / show dotfiles"),
        ("Ctrl+Shift+P", "Password mode (no key sounds; also automatic on password prompts)"),
        ("Ctrl+Shift+F5", "Replay startup"),
    ];
    egui::Window::new(title("SHORTCUTS"))
        .open(open)
        .frame(frame())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Grid::new("shortcuts_grid").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                for (k, a) in LIST {
                    ui.label(RichText::new(*k).font(theme::font(14.0, "mono")).color(theme::fg()));
                    lbl(ui, a);
                    ui.end_row();
                }
            });
        });
}

pub struct Fuzzy {
    pub q: String,
    pub sel: usize,
    pub fresh: bool,
}

impl Fuzzy {
    pub fn new() -> Self {
        Self { q: String::new(), sel: 0, fresh: true }
    }
}

/// Fuzzy finder sulla cartella corrente (max 5 risultati). Ritorna il nome scelto.
pub fn fuzzy_window(ctx: &egui::Context, open: &mut bool, f: &mut Fuzzy, names: &[(String, bool, PathBuf)]) -> Option<String> {
    if !*open {
        return None;
    }
    let mut pick: Option<String> = None;
    let mut close = false;
    egui::Window::new(title("FUZZY FINDER"))
        .open(open)
        .frame(frame())
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut f.q).hint_text("Search in current directory").desired_width(340.0));
            if f.fresh {
                r.request_focus();
                f.fresh = false;
            }
            let ql = f.q.to_lowercase();
            let mut res: Vec<&(String, bool, PathBuf)> = names.iter().filter(|n| n.0.to_lowercase().contains(&ql)).take(5).collect();
            res.sort_by_key(|n| !n.0.to_lowercase().starts_with(&ql));
            if ui.input(|i| i.key_pressed(Key::ArrowDown)) && f.sel + 1 < res.len() {
                f.sel += 1;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) && f.sel > 0 {
                f.sel -= 1;
            }
            if f.sel >= res.len() {
                f.sel = res.len().saturating_sub(1);
            }
            for (i, n) in res.iter().enumerate() {
                if ui.selectable_label(i == f.sel, RichText::new(short(&n.0, 48)).font(theme::font(15.0, "main")).color(theme::fg())).clicked() {
                    pick = Some(n.0.clone());
                }
            }
            if ui.input(|i| i.key_pressed(Key::Enter)) {
                if let Some(n) = res.get(f.sel) {
                    pick = Some(n.0.clone());
                }
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
        });
    if pick.is_some() || close {
        *open = false;
    }
    pick
}

pub struct ProcUi {
    pub key: u8, // 0 pid, 1 nome, 2 cpu, 3 memoria, 4 stato, 5 tempo
    pub asc: bool,
}

pub fn process_window(ctx: &egui::Context, open: &mut bool, ps: &mut ProcUi, mut rows: Vec<ProcRow>) {
    if !*open {
        return;
    }
    let k = ps.key;
    rows.sort_by(|a, b| {
        let o = match k {
            0 => a.pid.cmp(&b.pid),
            1 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            2 => a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal),
            3 => a.mem.cmp(&b.mem),
            4 => a.status.cmp(&b.status),
            _ => a.runtime.cmp(&b.runtime),
        };
        if ps.asc { o } else { o.reverse() }
    });
    egui::Window::new(title("ACTIVE PROCESSES"))
        .open(open)
        .frame(frame())
        .collapsible(false)
        .default_size([720.0, 460.0])
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("proc_grid").num_columns(6).spacing([18.0, 4.0]).striped(false).show(ui, |ui| {
                    for (i, h) in ["PID", "NAME", "CPU", "MEMORY", "STATE", "RUNTIME"].iter().enumerate() {
                        let arrow = if ps.key == i as u8 { if ps.asc { " ▲" } else { " ▼" } } else { "" };
                        if ui.selectable_label(false, RichText::new(format!("{h}{arrow}")).font(theme::font(14.0, "main")).color(theme::fg_a(0.7))).clicked() {
                            if ps.key == i as u8 {
                                ps.asc = !ps.asc;
                            } else {
                                ps.key = i as u8;
                                ps.asc = i == 0 || i == 1;
                            }
                        }
                    }
                    ui.end_row();
                    for r in &rows {
                        let t = |s: String| RichText::new(s).font(theme::font(13.0, "main")).color(theme::fg());
                        ui.label(t(r.pid.to_string()));
                        ui.label(t(short(&r.name, 32)));
                        ui.label(t(format!("{:.1}%", r.cpu)));
                        ui.label(t(bytes(r.mem)));
                        ui.label(t(r.status.clone()));
                        ui.label(t(format!("{}:{:02}:{:02}", r.runtime / 3600, (r.runtime / 60) % 60, r.runtime % 60)));
                        ui.end_row();
                    }
                });
            });
        });
}
