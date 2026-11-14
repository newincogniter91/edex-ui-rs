#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod audio;
mod boot;
mod data;
mod dialogs;
mod fsdisp;
mod hw;
mod icons;
mod net;
mod keyboard;
mod settings;
mod modules;
mod shell;
mod terminal;
mod theme;

use eframe::egui::{self, Align2, FontData, FontDefinitions, FontFamily, Key, Modifiers, Pos2, Rect, Sense, Stroke, Vec2};
use settings::Settings;
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
    settings: Settings,
    prev: Settings,
    show_settings: bool,
    show_shortcuts: bool,
    show_fuzzy: bool,
    fuzzy: dialogs::Fuzzy,
    show_procs: bool,
    procs_ui: dialogs::ProcUi,
    pass_mode: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        settings::apply_theme(&settings.theme);
        setup_fonts(&cc.egui_ctx);
        apply_style(&cc.egui_ctx);
        let mut terms: Vec<Option<Terminal>> = (0..5).map(|_| None).collect();
        terms[0] = Terminal::new(&cc.egui_ctx, &settings).ok();
        let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
        let mut fs = fsdisp::FsDisp::new();
        fs.hide_dotfiles = settings.hide_dotfiles;
        fs.list_view = settings.fs_list_view;
        let mut audio = audio::Audio::new();
        audio.enabled = settings.audio;
        audio.volume = settings.audio_volume;
        audio.feedback = !settings.disable_feedback_audio;
        Self {
            terms,
            active: 0,
            sys: modules::SysPanels::new(&settings),
            fs,
            kb: keyboard::Keyboard::new(&settings.keyboard),
            audio,
            tl: if settings.nointro { boot::Timeline::skipped() } else { boot::Timeline::new() },
            t0: None,
            fs_started: false,
            last_stdout: 0.0,
            user,
            prev: settings.clone(),
            settings,
            show_settings: false,
            show_shortcuts: false,
            show_fuzzy: false,
            fuzzy: dialogs::Fuzzy::new(),
            show_procs: false,
            procs_ui: dialogs::ProcUi { key: 2, asc: false },
            pass_mode: false,
        }
    }

    fn switch_tab(&mut self, ctx: &egui::Context, i: usize) {
        if i >= self.terms.len() {
            return;
        }
        self.audio.play(audio::Snd::Folder);
        if self.terms[i].is_none() {
            self.terms[i] = Terminal::new(ctx, &self.settings).ok();
        }
        if self.terms[i].is_some() {
            self.active = i;
        }
    }

    fn cycle_tab(&mut self, step: isize) {
        let n = self.terms.len() as isize;
        let mut i = self.active as isize;
        for _ in 0..n {
            i = (i + step).rem_euclid(n);
            if self.terms[i as usize].is_some() {
                self.active = i as usize;
                self.audio.play(audio::Snd::Folder);
                return;
            }
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
    v.override_text_color = Some(theme::fg());
    v.panel_fill = theme::bg();
    v.window_fill = theme::bg();
    v.extreme_bg_color = theme::bg();
    v.selection.bg_fill = theme::fg_a(0.3);
    v.selection.stroke = Stroke::new(1.0_f32, theme::fg());
    v.widgets.hovered.bg_fill = theme::grey();
    v.widgets.hovered.weak_bg_fill = theme::grey();
    v.widgets.inactive.bg_fill = theme::grey();
    v.widgets.inactive.weak_bg_fill = theme::grey();
    v.widgets.active.bg_fill = theme::grey();
    v.widgets.active.weak_bg_fill = theme::grey();
    v.widgets.open.bg_fill = theme::grey();
    v.widgets.open.weak_bg_fill = theme::grey();
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, theme::fg_a(0.3));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, theme::fg());
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, theme::fg());
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, theme::fg());
    v.window_stroke = Stroke::new(1.5_f32, theme::fg());
    v.window_rounding = 0.0.into();
    v.widgets.noninteractive.rounding = 0.0.into();
    v.widgets.inactive.rounding = 0.0.into();
    v.widgets.hovered.rounding = 0.0.into();
    v.widgets.active.rounding = 0.0.into();
    v.widgets.open.rounding = 0.0.into();
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.scroll = egui::style::ScrollStyle::solid();
        s.spacing.scroll.bar_width = 8.0;
    });
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::bg().to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sys.refresh();
        let now = ctx.input(|i| i.time);
        let skip = self.settings.nointro;
        let t0 = *self.t0.get_or_insert(now - if skip { 100.0 } else { 0.0 });

        // impostazioni cambiate: applica a caldo e salva
        if self.settings != self.prev {
            if self.settings.theme != self.prev.theme {
                settings::apply_theme(&self.settings.theme);
                apply_style(ctx);
            }
            if self.settings.keyboard != self.prev.keyboard {
                self.kb = keyboard::Keyboard::new(&self.settings.keyboard);
            }
            self.audio.enabled = self.settings.audio;
            self.audio.volume = self.settings.audio_volume;
            self.audio.feedback = !self.settings.disable_feedback_audio;
            self.sys.apply_settings(&self.settings);
            self.fs.hide_dotfiles = self.settings.hide_dotfiles;
            self.fs.list_view = self.settings.fs_list_view;
            self.settings.save();
            self.prev = self.settings.clone();
        }
        let t = now - t0;

        for s in self.tl.poll(t) {
            self.audio.play(s);
        }
        let ui_t = t - self.tl.ui_start; // < 0 durante boot e titolo
        let booting = ui_t < 0.0;
        let intro_done = ui_t >= boot::INTRO_LEN;
        let term_visible = ui_t >= 3.18;
        let modal_open = self.show_settings || self.show_shortcuts || self.show_fuzzy || self.show_procs;

        self.sys.globe_phase = (now as f32 * 0.35) % std::f32::consts::TAU;
        // durante l'avvio si ridisegna fluido; poi ~10 fps (il globo gira piano)
        let wait = if intro_done { 100 } else { 16 };
        ctx.request_repaint_after(Duration::from_millis(wait));

        // suoni dei tasti fisici (come nell'originale: keydown -> stdin, Invio rilasciato -> granted)
        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        if term_visible && !modal_open && !self.pass_mode {
            for e in &events {
                match e {
                    egui::Event::Key { pressed: true, .. } => self.audio.play_feedback(audio::Snd::Stdin),
                    egui::Event::Key { key: egui::Key::Enter, pressed: false, .. } => self.audio.play_feedback(audio::Snd::Granted),
                    _ => {}
                }
            }
            // output del terminale -> stdout, al massimo ogni 30 ms
            let mut got = false;
            for tm in self.terms.iter().flatten() {
                got |= tm.take_output();
            }
            if got && now - self.last_stdout > 0.03 {
                self.audio.play_feedback(audio::Snd::Stdout);
                self.last_stdout = now;
            }
        }

        // shell terminate (exit): la tab si svuota; se non resta nessun terminale, l'app si chiude
        if term_visible {
            for slot in self.terms.iter_mut() {
                if slot.as_mut().map(|t| t.exited()).unwrap_or(false) {
                    *slot = None;
                }
            }
            if self.terms.iter().all(|t| t.is_none()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if self.terms[self.active].is_none() {
                if let Some(i) = self.terms.iter().position(|t| t.is_some()) {
                    self.active = i;
                }
            }
        }

        // scorciatoie come nell'originale (consumano i tasti, così non arrivano al terminale)
        if term_visible {
            let cs = Modifiers { ctrl: true, shift: true, ..Default::default() };
            let mut acts: Vec<&'static str> = Vec::new();
            ctx.input_mut(|i| {
                for (k, a) in [
                    (Key::S, "SETTINGS"),
                    (Key::K, "SHORTCUTS"),
                    (Key::F, "FUZZY"),
                    (Key::L, "FSLIST"),
                    (Key::H, "FSDOT"),
                    (Key::P, "PASS"),
                    (Key::F5, "RELOAD"),
                    (Key::Tab, "PREV"),
                ] {
                    if i.consume_key(cs, k) {
                        acts.push(a);
                    }
                }
                if i.consume_key(Modifiers::CTRL, Key::Tab) {
                    acts.push("NEXT");
                }
                for (k, a) in [(Key::Num1, "TAB0"), (Key::Num2, "TAB1"), (Key::Num3, "TAB2"), (Key::Num4, "TAB3"), (Key::Num5, "TAB4")] {
                    if i.consume_key(Modifiers::CTRL, k) {
                        acts.push(a);
                    }
                }
            });
            for a in acts {
                match a {
                    "SETTINGS" => self.show_settings = !self.show_settings,
                    "SHORTCUTS" => self.show_shortcuts = !self.show_shortcuts,
                    "FUZZY" => {
                        self.show_fuzzy = !self.show_fuzzy;
                        self.fuzzy = dialogs::Fuzzy::new();
                    }
                    "FSLIST" => self.settings.fs_list_view = !self.settings.fs_list_view,
                    "FSDOT" => self.settings.hide_dotfiles = !self.settings.hide_dotfiles,
                    "PASS" => self.pass_mode = !self.pass_mode,
                    "RELOAD" => {
                        self.t0 = None;
                        self.tl = if self.settings.nointro { boot::Timeline::skipped() } else { boot::Timeline::new() };
                        self.fs_started = false;
                    }
                    "NEXT" => self.cycle_tab(1),
                    "PREV" => self.cycle_tab(-1),
                    t => self.switch_tab(ctx, t[3..].parse::<usize>().unwrap_or(0)),
                }
            }
        }

        // tasti fisici -> evidenzia sulla tastiera virtuale
        for e in events.iter().filter(|_| !modal_open && !self.pass_mode) {
            if let egui::Event::Text(tx) = e {
                self.kb.flash_key(tx.to_uppercase(), now);
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::bg()))
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
                let procs_rect = self.sys.left_column(&p, left, vh, &fade);
                if intro_done && ui.interact(procs_rect, ui.id().with("toplist"), Sense::click()).clicked() {
                    self.show_procs = true;
                }
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
                            if self.terms[i].is_none() {
                                "EMPTY".to_string()
                            } else if i == 0 {
                                "MAIN SHELL".to_string()
                            } else {
                                format!("SHELL #{}", i + 1)
                            }
                        })
                        .collect();
                    if let Some(i) = shell::tabs(ui, tabs_rect, &labels, self.active, vh) {
                        self.switch_tab(ctx, i);
                    }

                    let term_rect = Rect::from_min_max(
                        Pos2::new(shell.left() + vh * 0.74, shell.top() + tab_h + vh * 0.6),
                        Pos2::new(shell.right() - vh * 0.74, shell.bottom() - vh * 0.74),
                    );
                    let id = ui.id().with(("term", self.active));
                    let fsize = vh * 1.67 * (self.settings.term_font_size / 15.0);
                    let show_cursor = !self.settings.nocursor;
                    let input_ok = !modal_open;
                    if let Some(Some(tm)) = self.terms.get_mut(active) {
                        tm.ui(ui, term_rect, id, fsize, show_cursor, input_ok);
                    }
                }

                // ---- filesystem e tastiera (sotto la shell)
                if it >= 1.48 {
                    let bottom_top = screen.top() + vh * 67.3;
                    let fs_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 0.5, bottom_top), Vec2::new(vw * 43.0, vh * 31.0));
                    let kb_rect = Rect::from_min_size(Pos2::new(screen.left() + vw * 44.0, bottom_top), Vec2::new(vw * 55.5, vh * 31.0));

                    let ctrl_on = self.kb.ctrl_on();
                    let fs_act = self.fs.ui(ui, fs_rect, vh, now, ctrl_on);
                    if fsk_a < 1.0 {
                        p.rect_filled(fs_rect.expand(vh), 0.0, theme::bg_a(1.0 - fsk_a));
                    }
                    let key = self.kb.ui(ui, kb_rect, vh, now);
                    self.kb.veil(&p, kb_rect, vh, (it - 1.58).max(0.0), fsk_a);

                    for s in self.fs.sounds.drain(..) {
                        self.audio.play(s);
                    }
                    let ksounds: Vec<audio::Snd> = self.kb.sounds.drain(..).collect();
                    if term_visible && !modal_open {
                        if !self.pass_mode {
                            for s in ksounds {
                                self.audio.play_feedback(s);
                            }
                        }
                        match fs_act {
                            Some(fsdisp::FsAction::Cd(path)) => {
                                if let Some(Some(tm)) = self.terms.get_mut(active) {
                                    tm.send(&format!("cd \"{}\"\r", path.display()));
                                }
                            }
                            Some(fsdisp::FsAction::Open(path)) => fsdisp::open_path(&path),
                            Some(fsdisp::FsAction::WritePath(path)) => {
                                if let Some(Some(tm)) = self.terms.get_mut(active) {
                                    tm.send(&format!("\"{}\"", path.display()));
                                }
                            }
                            None => {}
                        }
                        if let Some(sq) = key {
                            if let Some(Some(tm)) = self.terms.get_mut(active) {
                                tm.send(&sq);
                            }
                        }
                    }
                }
            });

        // ---- finestre (impostazioni, scorciatoie, fuzzy finder, processi)
        dialogs::settings_window(ctx, &mut self.show_settings, &mut self.settings);
        dialogs::shortcuts_window(ctx, &mut self.show_shortcuts);
        let names = if self.show_fuzzy { self.fs.names() } else { Vec::new() };
        if let Some(name) = dialogs::fuzzy_window(ctx, &mut self.show_fuzzy, &mut self.fuzzy, &names) {
            let q = if name.contains(' ') { format!("\"{name}\"") } else { name };
            if let Some(Some(tm)) = self.terms.get_mut(self.active) {
                tm.send(&q);
            }
        }
        let rows = if self.show_procs { self.sys.process_rows() } else { Vec::new() };
        dialogs::process_window(ctx, &mut self.show_procs, &mut self.procs_ui, rows);
    }
}

/// Finché l'app è aperta Windows non va in sospensione né avvia lo screensaver / spegne lo schermo.
#[cfg(windows)]
fn keep_awake(on: bool) {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;
    let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED } else { ES_CONTINUOUS };
    unsafe {
        SetThreadExecutionState(flags);
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    keep_awake(true);
    let st = Settings::load();
    let mut vb = egui::ViewportBuilder::default();
    vb = if st.allow_windowed { vb.with_inner_size([1280.0, 800.0]).with_min_inner_size([960.0, 560.0]) } else { vb.with_fullscreen(true) };
    let options = eframe::NativeOptions {
        viewport: vb
            .with_icon(egui::IconData { rgba: include_bytes!("../assets/app_icon.rgba").to_vec(), width: 128, height: 128 })
            .with_title("eDEX-UI RS"),
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let res = eframe::run_native("eDEX-UI RS", options, Box::new(|cc| Box::new(App::new(cc))));
    #[cfg(windows)]
    keep_awake(false);
    res
}
