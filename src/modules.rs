use crate::theme::{self, fg_a, px, text, FG};
use eframe::egui::{Align2, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};

const HISTORY: usize = 60;

pub struct SysPanels {
    sys: System,
    nets: Networks,
    last_refresh: Instant,
    core_hist: Vec<VecDeque<f32>>,
    avg: f32,
    top: Vec<(u32, String, f32, f32)>,
    rx_rate: u64,
    tx_rate: u64,
    rx_total: u64,
    tx_total: u64,
    rx_hist: VecDeque<f32>,
    tx_hist: VecDeque<f32>,
    iface: String,
    pub globe_phase: f32,
}

impl SysPanels {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes();
        let n = sys.cpus().len().max(1);
        Self {
            sys,
            nets: Networks::new_with_refreshed_list(),
            last_refresh: Instant::now() - Duration::from_secs(10),
            core_hist: vec![VecDeque::from(vec![0.0; HISTORY]); n],
            avg: 0.0,
            top: Vec::new(),
            rx_rate: 0,
            tx_rate: 0,
            rx_total: 0,
            tx_total: 0,
            rx_hist: VecDeque::from(vec![0.0; HISTORY]),
            tx_hist: VecDeque::from(vec![0.0; HISTORY]),
            iface: String::from("--"),
            globe_phase: 0.0,
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

        for (i, c) in self.sys.cpus().iter().enumerate() {
            if let Some(h) = self.core_hist.get_mut(i) {
                push(h, c.cpu_usage());
            }
        }
        self.avg = self.sys.global_cpu_info().cpu_usage();

        let ncpu = self.sys.cpus().len().max(1) as f32;
        let total = self.sys.total_memory().max(1) as f32;
        let mut procs: Vec<(u32, String, f32, f32)> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, p)| (pid.as_u32(), p.name().to_string(), p.cpu_usage() / ncpu, p.memory() as f32 / total * 100.0))
            .collect();
        procs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        procs.truncate(5);
        self.top = procs;

        let (mut rx, mut tx, mut rxt, mut txt) = (0u64, 0u64, 0u64, 0u64);
        let mut best = (0u64, String::from("--"));
        for (name, d) in &self.nets {
            rx += d.received();
            tx += d.transmitted();
            rxt += d.total_received();
            txt += d.total_transmitted();
            let t = d.total_received() + d.total_transmitted();
            if t >= best.0 {
                best = (t, name.clone());
            }
        }
        self.iface = best.1;
        self.rx_rate = (rx as f32 / secs) as u64;
        self.tx_rate = (tx as f32 / secs) as u64;
        self.rx_total = rxt;
        self.tx_total = txt;
        push(&mut self.rx_hist, self.rx_rate as f32);
        push(&mut self.tx_hist, self.tx_rate as f32);
    }

    // ------------------------------------------------------------ colonne

    pub fn left_column(&self, p: &Painter, col: Rect, vh: f32) {
        let inner = col.shrink2(Vec2::new(vh * 1.39, 0.0));
        theme::title_bar(p, Rect::from_min_size(Pos2::new(inner.left(), col.top() + vh * 0.2), Vec2::new(inner.width(), vh * 1.6)), "PANEL", "SYSTEM", vh);
        let heights = [7.4, 5.6, 9.0, 21.0, 15.0, 15.0];
        let rects = distribute(inner, col.top() + vh * 3.2, col.bottom() - vh * 1.5, &heights, vh);
        self.clock(p, rects[0], vh);
        self.sysinfo(p, rects[1], vh);
        self.hardware(p, rects[2], vh);
        self.cpu(p, rects[3], vh);
        self.ram(p, rects[4], vh);
        self.toplist(p, rects[5], vh);
    }

    pub fn right_column(&self, p: &Painter, col: Rect, vh: f32) {
        let inner = col.shrink2(Vec2::new(vh * 1.39, 0.0));
        theme::title_bar(p, Rect::from_min_size(Pos2::new(inner.left(), col.top() + vh * 0.2), Vec2::new(inner.width(), vh * 1.6)), "PANEL", "NETWORK", vh);
        let globe_h = (inner.width() * 1.0 / vh + 6.0).min(40.0);
        let heights = [14.0, globe_h, 23.0];
        let rects = distribute(inner, col.top() + vh * 3.2, col.bottom() - vh * 1.5, &heights, vh);
        self.netstat(p, rects[0], vh);
        self.globe(p, rects[1], vh);
        self.conninfo(p, rects[2], vh);
    }

    // ------------------------------------------------------------ moduli sinistra

    fn clock(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        let now = chrono::Local::now();
        let s = now.format("%H:%M:%S").to_string();
        // ogni carattere in una cella fissa, come nell'originale
        let cell = vh * 2.5;
        let total = cell * s.chars().count() as f32 * 0.92;
        let mut x = r.center().x - total / 2.0 + cell * 0.46;
        for ch in s.chars() {
            let w = if ch == ':' { cell * 0.7 } else { cell };
            theme::text(p, x + w / 2.0, r.center().y + vh * 0.3, Align2::CENTER_CENTER, ch, vh * 4.0, FG, "light");
            x += w;
        }
    }

    fn sysinfo(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        let now = chrono::Local::now();
        let up = System::uptime();
        let uptime = format!("{}:{:02}:{:02}", up / 3600, (up / 60) % 60, up % 60);
        let cols: [(String, String); 4] = [
            (now.format("%Y").to_string(), now.format("%b %-d").to_string().to_uppercase()),
            ("UPTIME".into(), uptime),
            ("TYPE".into(), std::env::consts::OS.to_string()),
            ("POWER".into(), "WIRED".into()),
        ];
        cells(p, r, &cols, vh, 1.111);
    }

    fn hardware(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        let host = System::host_name().unwrap_or_else(|| "--".into());
        let os = System::name().unwrap_or_else(|| "--".into());
        let kernel = System::kernel_version().unwrap_or_else(|| "--".into());
        let cols: [(String, String); 3] = [("SYSTEM".into(), os), ("HOST".into(), host), ("KERNEL".into(), kernel)];
        cells(p, r, &cols, vh, 1.3);
    }

    fn cpu(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        let brand = self.sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
        header(p, r, "CPU USAGE", &short(&brand, 22), vh);

        let n = self.core_hist.len();
        let half = (n + 1) / 2;
        let w = r.width() * 0.76;
        let x0 = r.left() + (r.width() - w) / 2.0;
        let h = vh * 4.167;
        let mut y = r.top() + vh * 3.4;
        for (i, range) in [(0, half), (half, n)].iter().enumerate() {
            let cr = Rect::from_min_size(Pos2::new(x0, y), Vec2::new(w, h));
            dashed_box(p, cr, vh);
            for c in range.0..range.1 {
                line_chart(p, cr, &self.core_hist[c], 100.0, if c % 2 == 0 { 1.0 } else { 0.55 }, vh);
            }
            text_small(p, cr.left() - vh * 0.5, cr.center().y, Align2::RIGHT_CENTER, format!("#{}", i * half + 1), vh);
            y += h + vh * 0.9;
        }
        // riga inferiore
        let freq = self.sys.cpus().first().map(|c| c.frequency()).unwrap_or(0) as f32 / 1000.0;
        let cols: [(String, String); 3] = [
            ("AVG".into(), format!("{:.0}%", self.avg)),
            ("FREQ".into(), format!("{freq:.1} GHz")),
            ("TASKS".into(), format!("{}", self.sys.processes().len())),
        ];
        let row = Rect::from_min_max(Pos2::new(r.left(), y), r.right_bottom());
        p.add(Shape::dashed_line(
            &[row.left_top(), row.right_top()],
            Stroke::new(px(vh, 0.092), fg_a(0.3)),
            vh * 0.5,
            vh * 0.4,
        ));
        cells(p, row, &cols, vh, 1.3);
    }

    fn ram(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        let total = self.sys.total_memory().max(1) as f64;
        let used = self.sys.used_memory() as f64;
        let free = self.sys.free_memory() as f64;
        let avail = self.sys.available_memory() as f64;
        let gb = 1024.0 * 1024.0 * 1024.0;
        header(p, r, "MEMORY", &format!("USING {:.1} OUT OF {:.1} GB", used / gb, total / gb), vh);

        // matrice di 440 punti (40 x 11): attivi / disponibili / liberi
        let (cols, rows) = (40usize, 11usize);
        let n = cols * rows;
        let active = (((total - avail) / total) * n as f64).round() as usize;
        let free_n = ((free / total) * n as f64).round() as usize;
        let area = Rect::from_min_max(Pos2::new(r.left(), r.top() + vh * 3.4), Pos2::new(r.right(), r.top() + vh * 3.4 + vh * 6.3));
        let sx = area.width() / cols as f32;
        let sy = area.height() / rows as f32;
        for i in 0..n {
            let (cx, cy) = (i % cols, i / cols);
            let col = if i < active {
                FG
            } else if i >= n - free_n.min(n) {
                fg_a(0.18)
            } else {
                fg_a(0.5)
            };
            let c = Pos2::new(area.left() + sx * (cx as f32 + 0.5), area.top() + sy * (cy as f32 + 0.5));
            p.rect_filled(Rect::from_center_size(c, Vec2::new(sx * 0.5, sy * 0.45)), 0.0, col);
        }

        // swap
        let (st, su) = (self.sys.total_swap() as f64, self.sys.used_swap() as f64);
        let y = area.bottom() + vh * 1.2;
        text(p, r.left(), y, Align2::LEFT_CENTER, "SWAP", vh * 1.3, FG, "main");
        text(p, r.right(), y, Align2::RIGHT_CENTER, format!("{:.1} GB", su / gb), vh * 1.3, FG, "main");
        let bar = Rect::from_min_size(Pos2::new(r.left(), y + vh * 1.1), Vec2::new(r.width(), vh * 0.4));
        p.rect_filled(Rect::from_min_size(bar.left_center() - Vec2::new(0.0, vh * 0.125), Vec2::new(bar.width(), vh * 0.25)), 0.0, fg_a(0.25));
        let frac = if st > 0.0 { (su / st) as f32 } else { 0.0 };
        p.rect_filled(Rect::from_min_size(bar.left_top(), Vec2::new(bar.width() * frac, vh * 0.4)), 0.0, FG);
    }

    fn toplist(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        header(p, r, "TOP PROCESSES", "PID | NAME | CPU | MEM", vh);
        let mut y = r.top() + vh * 3.6;
        for (pid, name, cpu, mem) in &self.top {
            let sz = vh * 1.5;
            text(p, r.left(), y, Align2::LEFT_CENTER, pid, sz, FG, "main");
            text(p, r.left() + r.width() * 0.2, y, Align2::LEFT_CENTER, short(name, 13), sz, FG, "main");
            text(p, r.left() + r.width() * 0.76, y, Align2::RIGHT_CENTER, format!("{cpu:.0}%"), sz, FG, "main");
            text(p, r.right(), y, Align2::RIGHT_CENTER, format!("{mem:.0}%"), sz, FG, "main");
            y += vh * 2.1;
        }
    }

    // ------------------------------------------------------------ moduli destra

    fn netstat(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        header(p, r, "NETWORK STATUS", &format!("INTERFACE: {}", short(&self.iface, 16)), vh);
        let state = if self.nets.iter().next().is_some() { "ONLINE" } else { "OFFLINE" };
        let cols: [(String, String); 3] = [
            ("STATE".into(), state.into()),
            ("DOWN".into(), format!("{}/s", bytes(self.rx_rate))),
            ("UP".into(), format!("{}/s", bytes(self.tx_rate))),
        ];
        let row = Rect::from_min_max(Pos2::new(r.left(), r.top() + vh * 3.4), r.right_bottom());
        cells(p, row, &cols, vh, 1.3);
    }

    fn globe(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        header(p, r, "WORLD VIEW", "ENDPOINT LAT/LON: 0.0, 0.0", vh);
        let area = Rect::from_min_max(Pos2::new(r.left(), r.top() + vh * 3.0), r.right_bottom());
        let rad = (area.width().min(area.height()) * 0.46).max(10.0);
        let c = area.center();
        let ph = self.globe_phase;
        let st = Stroke::new(px(vh, 0.1), fg_a(0.55));
        // meridiani
        for k in 0..12 {
            let lon = ph + k as f32 * std::f32::consts::TAU / 12.0;
            let pts: Vec<Pos2> = (0..=24)
                .filter_map(|i| {
                    let lat = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 24.0;
                    let (x, y, z) = (lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos());
                    if z >= 0.0 { Some(Pos2::new(c.x + x * rad, c.y - y * rad)) } else { None }
                })
                .collect();
            if pts.len() > 1 {
                p.add(Shape::line(pts, st));
            }
        }
        // paralleli
        for k in 1..6 {
            let lat = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 6.0;
            let (y, rr) = (lat.sin() * rad, lat.cos() * rad);
            let pts: Vec<Pos2> = (0..=48)
                .filter_map(|i| {
                    let lon = ph + i as f32 * std::f32::consts::TAU / 48.0;
                    if lon.cos() >= 0.0 { Some(Pos2::new(c.x + rr * lon.sin(), c.y - y)) } else { None }
                })
                .collect();
            if pts.len() > 1 {
                p.add(Shape::line(pts, st));
            }
        }
        p.circle_stroke(c, rad, Stroke::new(px(vh, 0.14), fg_a(0.8)));
    }

    fn conninfo(&self, p: &Painter, r: Rect, vh: f32) {
        theme::module_frame(p, r, vh);
        header(p, r, "NETWORK TRAFFIC", "UP/DOWN, BYTES/S", vh);
        let h = vh * 8.7;
        let top = Rect::from_min_size(Pos2::new(r.left(), r.top() + vh * 3.4), Vec2::new(r.width(), h));
        let bot = Rect::from_min_size(Pos2::new(r.left(), top.bottom() + vh * 0.5), Vec2::new(r.width(), h));
        dashed_box(p, top, vh);
        dashed_box(p, bot, vh);
        let mx = |q: &VecDeque<f32>| q.iter().cloned().fold(1024.0, f32::max);
        line_chart(p, top, &self.tx_hist, mx(&self.tx_hist), 1.0, vh);
        line_chart(p, bot, &self.rx_hist, mx(&self.rx_hist), 1.0, vh);
        let tot = bytes(self.rx_total + self.tx_total);
        text(p, r.center().x, r.bottom() - vh * 1.2, Align2::CENTER_CENTER, format!("TOTAL {tot}"), vh * 2.0, FG, "main");
    }
}

// ---------------------------------------------------------------- helper

fn distribute(inner: Rect, top: f32, bottom: f32, heights: &[f32], vh: f32) -> Vec<Rect> {
    let sum: f32 = heights.iter().map(|h| h * vh).sum();
    let gap = ((bottom - top - sum) / (heights.len().max(2) - 1) as f32).max(vh * 0.8);
    let mut y = top;
    heights
        .iter()
        .map(|h| {
            let r = Rect::from_min_size(Pos2::new(inner.left(), y), Vec2::new(inner.width(), h * vh));
            y += h * vh + gap;
            r
        })
        .collect()
}

fn header(p: &Painter, r: Rect, title: &str, sub: &str, vh: f32) {
    let y = r.top() + vh * 1.5;
    text(p, r.left(), y, Align2::LEFT_CENTER, title, vh * 1.48, FG, "main");
    text(p, r.right(), y, Align2::RIGHT_CENTER, sub, vh * 1.2, fg_a(0.8), "main");
}

fn cells(p: &Painter, r: Rect, cols: &[(String, String)], vh: f32, size: f32) {
    let w = r.width() / cols.len() as f32;
    for (i, (k, v)) in cols.iter().enumerate() {
        let cx = r.left() + w * (i as f32 + 0.5);
        text(p, cx, r.top() + r.height() * 0.32 + vh * 0.2, Align2::CENTER_CENTER, short(k, 12), vh * size, FG, "main");
        text(p, cx, r.top() + r.height() * 0.68 + vh * 0.2, Align2::CENTER_CENTER, short(v, 12), vh * size * 0.92, fg_a(0.75), "main");
    }
}

fn dashed_box(p: &Painter, r: Rect, vh: f32) {
    let st = Stroke::new(px(vh, 0.092), fg_a(0.3));
    p.add(Shape::dashed_line(&[r.left_top(), r.right_top()], st, vh * 0.5, vh * 0.4));
    p.add(Shape::dashed_line(&[r.left_bottom(), r.right_bottom()], st, vh * 0.5, vh * 0.4));
}

fn line_chart(p: &Painter, r: Rect, hist: &VecDeque<f32>, max: f32, alpha: f32, vh: f32) {
    let n = hist.len().max(2) as f32 - 1.0;
    let pts: Vec<Pos2> = hist
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Pos2::new(
                r.left() + r.width() * i as f32 / n,
                r.bottom() - 1.0 - (r.height() - 2.0) * (v / max.max(0.001)).clamp(0.0, 1.0),
            )
        })
        .collect();
    p.add(Shape::line(pts, Stroke::new(px(vh, 0.14), fg_a(alpha))));
}

fn text_small(p: &Painter, x: f32, y: f32, a: Align2, s: String, vh: f32) {
    text(p, x, y, a, s, vh * 1.0, fg_a(0.6), "main");
}

fn push(q: &mut VecDeque<f32>, v: f32) {
    if q.len() >= HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}

pub fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

pub fn bytes(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{b} B") } else { format!("{v:.1} {}", U[i]) }
}
