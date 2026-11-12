use crate::theme;
use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};

const HISTORY: usize = 60;

pub struct SysPanels {
    sys: System,
    nets: Networks,
    last_refresh: Instant,
    cpu_hist: VecDeque<f32>,
    rx_hist: VecDeque<f32>,
    tx_hist: VecDeque<f32>,
    top: Vec<(String, f32, u64)>,
    rx_rate: u64,
    tx_rate: u64,
    ifaces: Vec<(String, u64, u64)>,
}

impl SysPanels {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes();
        Self {
            sys,
            nets: Networks::new_with_refreshed_list(),
            last_refresh: Instant::now() - Duration::from_secs(10),
            cpu_hist: VecDeque::from(vec![0.0; HISTORY]),
            rx_hist: VecDeque::from(vec![0.0; HISTORY]),
            tx_hist: VecDeque::from(vec![0.0; HISTORY]),
            top: Vec::new(),
            rx_rate: 0,
            tx_rate: 0,
            ifaces: Vec::new(),
        }
    }

    /// Aggiorna le metriche al massimo una volta al secondo.
    pub fn refresh(&mut self) {
        let dt = self.last_refresh.elapsed();
        if dt < Duration::from_millis(1000) {
            return;
        }
        let secs = dt.as_secs_f32().max(0.1);
        self.last_refresh = Instant::now();

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes();
        self.nets.refresh();

        push(&mut self.cpu_hist, self.sys.global_cpu_info().cpu_usage());

        let ncpu = self.sys.cpus().len().max(1) as f32;
        let mut procs: Vec<(String, f32, u64)> = self
            .sys
            .processes()
            .values()
            .map(|p| (p.name().to_string(), p.cpu_usage() / ncpu, p.memory()))
            .collect();
        procs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        procs.truncate(6);
        self.top = procs;

        let (mut rx, mut tx) = (0u64, 0u64);
        self.ifaces.clear();
        for (name, d) in &self.nets {
            rx += d.received();
            tx += d.transmitted();
            self.ifaces.push((name.clone(), d.total_received(), d.total_transmitted()));
        }
        self.rx_rate = (rx as f32 / secs) as u64;
        self.tx_rate = (tx as f32 / secs) as u64;
        push(&mut self.rx_hist, self.rx_rate as f32);
        push(&mut self.tx_hist, self.tx_rate as f32);
    }

    pub fn left(&mut self, ui: &mut egui::Ui) {
        section(ui, "CLOCK", |ui| {
            let now = chrono::Local::now();
            ui.label(RichText::new(now.format("%H:%M:%S").to_string()).size(34.0).color(theme::FG));
            ui.label(RichText::new(now.format("%A %d %B %Y").to_string()).color(theme::DIM));
        });

        section(ui, "SYSTEM", |ui| {
            kv(ui, "HOST", &System::host_name().unwrap_or_else(|| "?".into()));
            kv(ui, "OS", &System::long_os_version().unwrap_or_else(|| "?".into()));
            kv(ui, "UPTIME", &fmt_uptime(System::uptime()));
        });

        section(ui, "CPU", |ui| {
            let brand = self.sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
            ui.label(RichText::new(brand).size(11.0).color(theme::DIM));
            let cur = *self.cpu_hist.back().unwrap_or(&0.0);
            kv(ui, "LOAD", &format!("{cur:.0}%  ({} core)", self.sys.cpus().len()));
            spark(ui, &self.cpu_hist, 100.0, 46.0);
        });

        section(ui, "MEMORY", |ui| {
            let total = self.sys.total_memory();
            let used = self.sys.used_memory();
            let frac = if total > 0 { used as f32 / total as f32 } else { 0.0 };
            kv(ui, "USED", &format!("{} / {}", fmt_bytes(used), fmt_bytes(total)));
            bar(ui, frac);
        });

        section(ui, "TOP PROCESSES", |ui| {
            for (name, cpu, mem) in &self.top {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(truncate(name, 16)).size(12.0).color(theme::FG));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{cpu:>4.1}%  {}", fmt_bytes(*mem)))
                                .size(12.0)
                                .color(theme::DIM),
                        );
                    });
                });
            }
        });
    }

    pub fn network(&mut self, ui: &mut egui::Ui) {
        section(ui, "NETWORK", |ui| {
            kv(ui, "DOWN", &format!("{}/s", fmt_bytes(self.rx_rate)));
            spark(ui, &self.rx_hist, self.rx_hist.iter().cloned().fold(1024.0, f32::max), 30.0);
            kv(ui, "UP", &format!("{}/s", fmt_bytes(self.tx_rate)));
            spark(ui, &self.tx_hist, self.tx_hist.iter().cloned().fold(1024.0, f32::max), 30.0);
            for (name, rx, tx) in self.ifaces.iter().take(3) {
                ui.label(
                    RichText::new(format!("{}  ↓{} ↑{}", truncate(name, 14), fmt_bytes(*rx), fmt_bytes(*tx)))
                        .size(11.0)
                        .color(theme::DIM),
                );
            }
        });
    }
}

// ---------------------------------------------------------------- filesystem

pub struct FsBrowser {
    cwd: Option<PathBuf>, // None = elenco unità (solo Windows)
    entries: Vec<(String, bool)>,
}

impl FsBrowser {
    pub fn new() -> Self {
        let start = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut f = Self { cwd: Some(start), entries: Vec::new() };
        f.reload();
        f
    }

    fn reload(&mut self) {
        self.entries.clear();
        match &self.cwd {
            Some(p) => {
                if let Ok(rd) = std::fs::read_dir(p) {
                    for e in rd.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        self.entries.push((name, is_dir));
                    }
                }
                self.entries
                    .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
            }
            None => {
                for l in b'A'..=b'Z' {
                    let d = format!("{}:\\", l as char);
                    if std::path::Path::new(&d).exists() {
                        self.entries.push((d, true));
                    }
                }
            }
        }
    }

    /// Ritorna Some(path) se l'utente è entrato in una cartella (per sincronizzare il terminale).
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut navigate: Option<Option<PathBuf>> = None;
        let mut result = None;

        let title = match &self.cwd {
            Some(p) => p.display().to_string(),
            None => "DRIVES".into(),
        };

        section(ui, "FILESYSTEM", |ui| {
            ui.label(RichText::new(truncate(&title, 34)).size(11.0).color(theme::DIM));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if self.cwd.is_some() && ui.selectable_label(false, RichText::new("▲ ..").color(theme::FG)).clicked() {
                    let parent = self.cwd.as_ref().and_then(|p| p.parent().map(|x| x.to_path_buf()));
                    navigate = Some(parent);
                }
                for (name, is_dir) in &self.entries {
                    let icon = if *is_dir { "▸" } else { "·" };
                    let color = if *is_dir { theme::FG } else { theme::DIM };
                    let r = ui.selectable_label(false, RichText::new(format!("{icon} {name}")).color(color));
                    if r.clicked() && *is_dir {
                        let next = match &self.cwd {
                            Some(p) => p.join(name),
                            None => PathBuf::from(name),
                        };
                        navigate = Some(Some(next));
                    }
                }
            });
        });

        if let Some(n) = navigate {
            if let Some(p) = &n {
                result = Some(p.clone());
            }
            self.cwd = n;
            self.reload();
        }
        result
    }
}

// ---------------------------------------------------------------- helpers

pub fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .stroke(Stroke::new(1.0, theme::FG.gamma_multiply(0.35)))
        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
        .outer_margin(egui::Margin::symmetric(0.0, 3.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(11.0).color(theme::DIM).strong());
            ui.add_space(2.0);
            add(ui);
        });
}

fn kv(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(k).size(12.0).color(theme::DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(truncate(v, 28)).size(12.0).color(theme::FG));
        });
    });
}

fn bar(ui: &mut egui::Ui, frac: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), Sense::hover());
    let p = ui.painter();
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::FG.gamma_multiply(0.5)));
    let mut fill = rect.shrink(2.0);
    fill.set_width(fill.width() * frac.clamp(0.0, 1.0));
    p.rect_filled(fill, 0.0, theme::FG);
}

fn spark(ui: &mut egui::Ui, hist: &VecDeque<f32>, max: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::GREY));
    let n = hist.len().max(2) as f32 - 1.0;
    let pts: Vec<Pos2> = hist
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + rect.width() * i as f32 / n;
            let y = rect.bottom() - 2.0 - (rect.height() - 4.0) * (v / max.max(0.001)).clamp(0.0, 1.0);
            Pos2::new(x, y)
        })
        .collect();
    p.add(egui::Shape::line(pts, Stroke::new(1.5, theme::FG)));
}

pub fn header(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.text(
        Pos2::new(rect.left() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        "eDEX-UI · RS",
        FontId::proportional(14.0),
        theme::FG,
    );
    p.text(
        Pos2::new(rect.right() - 6.0, rect.center().y),
        Align2::RIGHT_CENTER,
        format!("{}  ·  v{}", std::env::consts::OS.to_uppercase(), env!("CARGO_PKG_VERSION")),
        FontId::proportional(11.0),
        theme::DIM,
    );
    let line = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - 1.0), rect.right_bottom());
    p.rect_filled(line, 0.0, theme::FG.gamma_multiply(0.35));
}

fn push(q: &mut VecDeque<f32>, v: f32) {
    if q.len() >= HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn fmt_bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

fn fmt_uptime(s: u64) -> String {
    let (d, h, m) = (s / 86400, (s / 3600) % 24, (s / 60) % 60);
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else {
        format!("{h}h {m}m")
    }
}
