use crate::icons::Icons;
use crate::modules::short;
use crate::theme::{self, fg_a, px, text, FG};
use eframe::egui::{self, Align2, Painter, Pos2, Rect, Sense, Stroke, Vec2};
use std::path::{Path, PathBuf};
use sysinfo::Disks;

pub struct FsDisp {
    cwd: Option<PathBuf>, // None = elenco unità
    entries: Vec<(String, bool)>,
    disks: Disks,
    icons: Icons,
    drive_kinds: std::collections::HashMap<String, u8>,
}

impl FsDisp {
    pub fn new() -> Self {
        let start = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut f = Self { cwd: Some(start), entries: Vec::new(), disks: Disks::new_with_refreshed_list(), icons: Icons::new(), drive_kinds: Default::default() };
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
                self.entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
            }
            None => {
                // le unità si rileggono ogni volta che si apre la vista
                self.disks.refresh_list();
                self.drive_kinds.clear();
                for (n, k) in list_drives(&self.disks) {
                    self.drive_kinds.insert(n.clone(), k);
                    self.entries.push((n, true));
                }
            }
        }
    }

    /// Disegna il filesystem; ritorna Some(path) quando si entra in una cartella.
    pub fn ui(&mut self, ui: &mut egui::Ui, r: Rect, vh: f32) -> Option<PathBuf> {
        let p = ui.painter_at(r.expand(vh));
        let title = match &self.cwd {
            Some(c) => c.display().to_string(),
            None => "DISKS".into(),
        };
        theme::title_bar(&p, Rect::from_min_size(r.min, Vec2::new(r.width(), vh * 1.6)), "FILESYSTEM", &short(&title, 60), vh);

        let area = Rect::from_min_max(Pos2::new(r.left(), r.top() + vh * 2.2), Pos2::new(r.right(), r.bottom() - vh * 3.6));
        let tile = vh * 8.5;
        let gap = vh * 1.0;
        let per_row = (((area.width() + gap) / (tile + gap)).floor() as usize).max(1);

        let mut nav: Option<Option<PathBuf>> = None;
        let mut items: Vec<(String, u8)> = Vec::new(); // 0 file, 1 dir, 2 su, 3 dischi, 4 disco
        if self.cwd.is_some() {
            items.push(("DISKS".into(), 3));
            items.push(("..".into(), 2));
        }
        for (n, d) in &self.entries {
            items.push((n.clone(), if self.cwd.is_none() { self.drive_kinds.get(n).copied().unwrap_or(4) } else if *d { 1 } else { 0 }));
        }

        let rows = (items.len() + per_row - 1) / per_row;
        let content_h = rows as f32 * (tile + gap);
        let mut child = ui.child_ui(area, egui::Layout::top_down(egui::Align::Min));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
            let (resp_rect, _) = ui.allocate_exact_size(Vec2::new(area.width(), content_h), Sense::hover());
            let pp = ui.painter_at(area);
            for (i, (name, kind)) in items.iter().enumerate() {
                let (cx, cy) = (i % per_row, i / per_row);
                let tr = Rect::from_min_size(
                    Pos2::new(resp_rect.left() + cx as f32 * (tile + gap), resp_rect.top() + cy as f32 * (tile + gap)),
                    Vec2::splat(tile),
                );
                let resp = ui.interact(tr, ui.id().with(("fs", i)), Sense::click());
                if resp.hovered() {
                    pp.rect_filled(tr, 0.0, fg_a(0.12));
                }
                let idx = match kind {
                    0 | 1 => self.icons.for_entry(name, *kind == 1),
                    2 => self.icons.core("up"),
                    3 => self.icons.core("showDisks"),
                    5 => self.icons.core("usb"),
                    6 => self.icons.core("rom"),
                    _ => self.icons.core("disk"),
                };
                let isz = if *kind >= 2 { vh * 4.0 } else { vh * 5.0 };
                if let Some(tid) = self.icons.texture(ui.ctx(), idx) {
                    let ir = Rect::from_center_size(Pos2::new(tr.center().x, tr.top() + vh * 3.4), Vec2::splat(isz));
                    pp.image(tid, ir, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), egui::Color32::WHITE);
                }
                text(&pp, tr.center().x, tr.bottom() - vh * 1.2, Align2::CENTER_CENTER, short(name, 12), vh * 1.3, FG, "main");
                if resp.clicked() {
                    match kind {
                        3 => nav = Some(None),
                        2 => nav = Some(self.cwd.as_ref().and_then(|c| c.parent().map(|x| x.to_path_buf()))),
                        1 | 4 | 5 | 6 => {
                            nav = Some(Some(match &self.cwd {
                                Some(c) => c.join(name),
                                None => PathBuf::from(name),
                            }))
                        }
                        _ => {}
                    }
                }
            }
        });

        // barra dello spazio disco
        self.space_bar(&p, Rect::from_min_max(Pos2::new(r.left(), r.bottom() - vh * 3.0), r.right_bottom()), vh);

        let mut result = None;
        if let Some(n) = nav {
            if let Some(path) = &n {
                result = Some(path.clone());
            }
            self.cwd = n;
            self.reload();
        }
        result
    }

    fn space_bar(&self, p: &Painter, r: Rect, vh: f32) {
        let cwd = match &self.cwd {
            Some(c) => c.clone(),
            None => return,
        };
        let mut best: Option<(&sysinfo::Disk, usize)> = None;
        for d in self.disks.list() {
            let mp = d.mount_point();
            if path_starts(&cwd, mp) {
                let l = mp.as_os_str().len();
                if best.map(|b| l >= b.1).unwrap_or(true) {
                    best = Some((d, l));
                }
            }
        }
        if let Some((d, _)) = best {
            let total = d.total_space().max(1) as f32;
            let used = total - d.available_space() as f32;
            let frac = used / total;
            let b = Rect::from_min_size(r.min, Vec2::new(r.width(), vh * 2.6));
            p.rect_stroke(b, 0.0, Stroke::new(px(vh, 0.2), fg_a(0.5)));
            p.rect_filled(Rect::from_min_size(b.min, Vec2::new(b.width() * frac, b.height())), 0.0, fg_a(0.35));
            text(
                p,
                b.center().x,
                b.center().y,
                Align2::CENTER_CENTER,
                format!("Mount {} used {:.0}%", short(&d.mount_point().to_string_lossy(), 20), frac * 100.0),
                vh * 1.3,
                FG,
                "main",
            );
        }
    }
}

fn path_starts(p: &Path, base: &Path) -> bool {
    p.starts_with(base)
}

/// Elenco delle unità presenti in questo momento (nome, tipo: 4 disco, 5 rimovibile, 6 ottico).
#[cfg(windows)]
fn list_drives(_d: &Disks) -> Vec<(String, u8)> {
    extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(root: *const u16) -> u32;
    }
    let mask = unsafe { GetLogicalDrives() };
    let mut v = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) != 0 {
            let name = format!("{}:\\", (b'A' + i as u8) as char);
            let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let t = unsafe { GetDriveTypeW(wide.as_ptr()) };
            let kind = match t {
                2 => 5,
                5 => 6,
                _ => 4,
            };
            v.push((name, kind));
        }
    }
    v
}

#[cfg(not(windows))]
fn list_drives(d: &Disks) -> Vec<(String, u8)> {
    d.list().iter().map(|x| (x.mount_point().to_string_lossy().to_string(), 4)).collect()
}
