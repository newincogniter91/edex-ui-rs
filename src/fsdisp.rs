use crate::audio::Snd;
use crate::icons::Icons;
use crate::modules::{bytes, short};
use crate::theme::{self, fg, fg_a, px, text};
use eframe::egui::{self, Align2, Painter, Pos2, Rect, Sense, Stroke, Vec2};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sysinfo::Disks;

/// Cosa deve fare l'app dopo un clic nel file manager.
pub enum FsAction {
    Cd(PathBuf),
    Open(PathBuf),
    WritePath(PathBuf),
}

struct Ent {
    name: String,
    is_dir: bool,
    hidden: bool,
    size: Option<u64>,
    modified: Option<SystemTime>,
}

struct Item {
    name: String,
    kind: u8, // 0 file, 1 dir, 2 su, 3 dischi, 4 disco, 5 rimovibile, 6 ottico
    path: Option<PathBuf>,
    size: Option<u64>,
    modified: Option<SystemTime>,
}

pub struct FsDisp {
    cwd: Option<PathBuf>, // None = elenco unità
    entries: Vec<Ent>,
    disks: Disks,
    icons: Icons,
    drive_kinds: std::collections::HashMap<String, u8>,
    reveal_t0: f64,
    revealed: usize,
    restart_pending: bool,
    pub sounds: Vec<Snd>,
    pub hide_dotfiles: bool,
    pub list_view: bool,
    sort_key: u8, // 0 nome, 1 dimensione, 2 data
    sort_asc: bool,
}

impl FsDisp {
    pub fn new() -> Self {
        let start = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut f = Self {
            cwd: Some(start),
            entries: Vec::new(),
            disks: Disks::new_with_refreshed_list(),
            icons: Icons::new(),
            drive_kinds: Default::default(),
            reveal_t0: 1e18,
            revealed: 0,
            restart_pending: false,
            sounds: Vec::new(),
            hide_dotfiles: false,
            list_view: false,
            sort_key: 0,
            sort_asc: true,
        };
        f.reload();
        f
    }

    /// Riparte la comparsa sequenziale delle voci (una ogni 30 ms, con suono).
    pub fn restart(&mut self, now: f64) {
        self.reveal_t0 = now;
        self.revealed = 0;
    }

    /// Voci della cartella corrente (per il fuzzy finder): nome, è cartella, percorso.
    pub fn names(&self) -> Vec<(String, bool, PathBuf)> {
        match &self.cwd {
            Some(c) => self
                .entries
                .iter()
                .filter(|e| !(self.hide_dotfiles && e.hidden))
                .map(|e| (e.name.clone(), e.is_dir, c.join(&e.name)))
                .collect(),
            None => Vec::new(),
        }
    }

    fn sort(&mut self) {
        let (k, asc) = (self.sort_key, self.sort_asc);
        self.entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then_with(|| {
                let o = match k {
                    1 => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
                    2 => a.modified.cmp(&b.modified),
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                };
                if asc { o } else { o.reverse() }
            })
        });
    }

    fn reload(&mut self) {
        self.restart_pending = true;
        self.entries.clear();
        match &self.cwd {
            Some(p) => {
                if let Ok(rd) = std::fs::read_dir(p) {
                    for e in rd.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        let md = e.metadata().ok();
                        let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or_else(|| e.file_type().map(|t| t.is_dir()).unwrap_or(false));
                        let hidden = name.starts_with('.') || is_hidden_attr(md.as_ref());
                        self.entries.push(Ent {
                            hidden,
                            size: if is_dir { None } else { md.as_ref().map(|m| m.len()) },
                            modified: md.as_ref().and_then(|m| m.modified().ok()),
                            is_dir,
                            name,
                        });
                    }
                }
                self.sort();
            }
            None => {
                // le unità si rileggono ogni volta che si apre la vista
                self.disks.refresh_list();
                self.drive_kinds.clear();
                for (n, k) in list_drives(&self.disks) {
                    self.drive_kinds.insert(n.clone(), k);
                    self.entries.push(Ent { name: n, is_dir: true, hidden: false, size: None, modified: None });
                }
            }
        }
    }

    fn items(&self) -> Vec<Item> {
        let mut v = Vec::new();
        if self.cwd.is_some() {
            v.push(Item { name: "DISKS".into(), kind: 3, path: None, size: None, modified: None });
            v.push(Item { name: "..".into(), kind: 2, path: None, size: None, modified: None });
        }
        for e in &self.entries {
            if self.hide_dotfiles && e.hidden {
                continue;
            }
            let kind = if self.cwd.is_none() {
                self.drive_kinds.get(&e.name).copied().unwrap_or(4)
            } else if e.is_dir {
                1
            } else {
                0
            };
            let path = match &self.cwd {
                Some(c) => c.join(&e.name),
                None => PathBuf::from(&e.name),
            };
            v.push(Item { name: e.name.clone(), kind, path: Some(path), size: e.size, modified: e.modified });
        }
        v
    }

    /// Disegna il filesystem; ritorna l'azione richiesta dall'utente (se c'è).
    pub fn ui(&mut self, ui: &mut egui::Ui, r: Rect, vh: f32, now: f64, ctrl_on: bool) -> Option<FsAction> {
        if self.restart_pending && self.reveal_t0 < 1e17 {
            self.restart(now);
        }
        self.restart_pending = false;

        let p = ui.painter_at(r.expand(vh));
        let title = match &self.cwd {
            Some(c) => c.display().to_string(),
            None => "DISKS".into(),
        };
        theme::title_bar(&p, Rect::from_min_size(r.min, Vec2::new(r.width(), vh * 1.6)), "FILESYSTEM", &short(&title, 60), vh);

        let items = self.items();
        let mut area = Rect::from_min_max(Pos2::new(r.left(), r.top() + vh * 2.2), Pos2::new(r.right(), r.bottom() - vh * 3.6));
        let mut nav: Option<Option<PathBuf>> = None;
        let mut action: Option<FsAction> = None;
        let cwd_c = self.cwd.clone();

        // intestazione colonne (vista a lista): clic per ordinare
        let row_h = vh * 2.8;
        let (c_size, c_date) = (area.width() * 0.62, area.width() - vh);
        if self.list_view {
            let hr = Rect::from_min_size(area.min, Vec2::new(area.width(), row_h));
            let heads = [("NAME", area.left() + vh * 3.6, Align2::LEFT_CENTER, 0u8), ("SIZE", area.left() + c_size, Align2::RIGHT_CENTER, 1u8), ("MODIFIED", area.left() + c_date, Align2::RIGHT_CENTER, 2u8)];
            for (label, x, al, key) in heads {
                let arrow = if self.sort_key == key { if self.sort_asc { " ▲" } else { " ▼" } } else { "" };
                text(&p, x, hr.center().y, al, format!("{label}{arrow}"), vh * 1.2, fg_a(0.6), "main");
                let w = vh * 9.0;
                let cr = if al == Align2::LEFT_CENTER { Rect::from_min_size(Pos2::new(x, hr.top()), Vec2::new(w, row_h)) } else { Rect::from_min_size(Pos2::new(x - w, hr.top()), Vec2::new(w, row_h)) };
                if ui.interact(cr, ui.id().with(("fshead", key)), Sense::click()).clicked() {
                    if self.sort_key == key {
                        self.sort_asc = !self.sort_asc;
                    } else {
                        self.sort_key = key;
                        self.sort_asc = true;
                    }
                    self.sort();
                }
            }
            p.line_segment([hr.left_bottom(), hr.right_bottom()], Stroke::new(px(vh, 0.092), fg_a(0.3)));
            area.min.y += row_h;
        }

        let tile = vh * 8.5;
        let gap = vh * 1.0;
        let per_row = (((area.width() + gap) / (tile + gap)).floor() as usize).max(1);
        let list = self.list_view;
        let content_h = if list { items.len() as f32 * row_h } else { ((items.len() + per_row - 1) / per_row) as f32 * (tile + gap) };

        let mut child = ui.child_ui(area, egui::Layout::top_down(egui::Align::Min));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
            let (resp_rect, _) = ui.allocate_exact_size(Vec2::new(area.width(), content_h), Sense::hover());
            let pp = ui.painter_at(area);
            let shown = (((now - self.reveal_t0) / 0.03).floor().max(0.0) as usize).min(items.len());
            if shown > self.revealed {
                self.sounds.push(Snd::Folder);
                self.revealed = shown;
            }
            if shown < items.len() && self.reveal_t0 < 1e17 {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
            }
            for (i, it) in items.iter().enumerate().take(shown) {
                let tr = if list {
                    Rect::from_min_size(Pos2::new(resp_rect.left(), resp_rect.top() + i as f32 * row_h), Vec2::new(area.width(), row_h))
                } else {
                    let (cx, cy) = (i % per_row, i / per_row);
                    Rect::from_min_size(
                        Pos2::new(resp_rect.left() + cx as f32 * (tile + gap), resp_rect.top() + cy as f32 * (tile + gap)),
                        Vec2::splat(tile),
                    )
                };
                if tr.bottom() < area.top() || tr.top() > area.bottom() {
                    continue;
                }
                let resp = ui.interact(tr, ui.id().with(("fs", i)), Sense::click());
                if resp.hovered() {
                    pp.rect_filled(tr, 0.0, fg_a(0.12));
                }
                let idx = match it.kind {
                    0 | 1 => self.icons.for_entry(&it.name, it.kind == 1),
                    2 => self.icons.core("up"),
                    3 => self.icons.core("showDisks"),
                    5 => self.icons.core("usb"),
                    6 => self.icons.core("rom"),
                    _ => self.icons.core("disk"),
                };
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                if list {
                    if let Some(tid) = self.icons.texture(ui.ctx(), idx) {
                        let ir = Rect::from_center_size(Pos2::new(tr.left() + vh * 1.8, tr.center().y), Vec2::splat(vh * 2.0));
                        pp.image(tid, ir, uv, fg());
                    }
                    text(&pp, tr.left() + vh * 3.6, tr.center().y, Align2::LEFT_CENTER, short(&it.name, 48), vh * 1.4, fg(), "main");
                    if let Some(s) = it.size {
                        text(&pp, tr.left() + c_size, tr.center().y, Align2::RIGHT_CENTER, bytes(s), vh * 1.3, fg_a(0.75), "main");
                    }
                    if let Some(m) = it.modified {
                        let d: chrono::DateTime<chrono::Local> = m.into();
                        text(&pp, tr.left() + c_date, tr.center().y, Align2::RIGHT_CENTER, d.format("%Y-%m-%d %H:%M"), vh * 1.3, fg_a(0.75), "main");
                    }
                } else {
                    let isz = if it.kind >= 2 { vh * 4.0 } else { vh * 5.0 };
                    if let Some(tid) = self.icons.texture(ui.ctx(), idx) {
                        let ir = Rect::from_center_size(Pos2::new(tr.center().x, tr.top() + vh * 3.4), Vec2::splat(isz));
                        pp.image(tid, ir, uv, fg());
                    }
                    text(&pp, tr.center().x, tr.bottom() - vh * 1.2, Align2::CENTER_CENTER, short(&it.name, 12), vh * 1.3, fg(), "main");
                }

                if resp.clicked() {
                    match it.kind {
                        3 => nav = Some(None),
                        2 => nav = Some(cwd_c.as_ref().and_then(|c| c.parent().map(|x| x.to_path_buf()))),
                        1 | 4 | 5 | 6 => {
                            if let Some(path) = &it.path {
                                if ctrl_on && it.kind == 1 {
                                    action = Some(FsAction::WritePath(path.clone()));
                                } else {
                                    nav = Some(Some(path.clone()));
                                }
                            }
                        }
                        _ => {
                            if let Some(path) = &it.path {
                                action = Some(if ctrl_on { FsAction::WritePath(path.clone()) } else { FsAction::Open(path.clone()) });
                            }
                        }
                    }
                }
            }
        });

        // barra dello spazio disco
        self.space_bar(&p, Rect::from_min_max(Pos2::new(r.left(), r.bottom() - vh * 3.0), r.right_bottom()), vh);

        if let Some(n) = nav {
            if let Some(path) = &n {
                action = Some(FsAction::Cd(path.clone()));
            }
            self.cwd = n;
            self.reload();
        }
        action
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
                fg(),
                "main",
            );
        }
    }
}

fn path_starts(p: &Path, base: &Path) -> bool {
    p.starts_with(base)
}

#[cfg(windows)]
fn is_hidden_attr(md: Option<&std::fs::Metadata>) -> bool {
    use std::os::windows::fs::MetadataExt;
    md.map(|m| m.file_attributes() & 0x2 != 0).unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden_attr(_md: Option<&std::fs::Metadata>) -> bool {
    false
}

/// Apre un file con l'applicazione predefinita del sistema.
pub fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd").arg("/C").arg("start").arg("").arg(path).creation_flags(0x0800_0000).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
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
