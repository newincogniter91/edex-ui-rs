use crate::theme;
use eframe::egui::{self, Color32, EventFilter, FontId, Key, Pos2, Rect, Sense, Vec2};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    cols: u16,
    rows: u16,
    scroll: usize,
    out: Arc<AtomicBool>,
}

impl Terminal {
    pub fn new(ctx: &egui::Context) -> std::io::Result<Self> {
        let (cols, rows) = (100u16, 30u16);
        let pair = native_pty_system()
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let cmd = default_shell();
        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 3000)));
        let p2 = parser.clone();
        let ctx2 = ctx.clone();
        let out = Arc::new(AtomicBool::new(false));
        let out2 = out.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut p) = p2.lock() {
                            p.process(&buf[..n]);
                        }
                        out2.store(true, Ordering::Relaxed);
                        ctx2.request_repaint();
                    }
                }
            }
        });

        Ok(Self { parser, writer, master: pair.master, cols, rows, scroll: 0, out })
    }

    /// True se è arrivato output dall'ultima chiamata (per il suono stdout).
    pub fn take_output(&self) -> bool {
        self.out.swap(false, Ordering::Relaxed)
    }

    pub fn send(&mut self, s: &str) {
        let _ = self.writer.write_all(s.as_bytes());
        let _ = self.writer.flush();
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows || cols < 2 || rows < 2 {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        if let Ok(mut p) = self.parser.lock() {
            p.set_size(rows, cols);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, rect: Rect, id: egui::Id, size: f32) {
        let font = theme::font(size, "mono");
        let (cw, rh) = ui.fonts(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let resp = ui.interact(rect, id, Sense::click());

        // il terminale tiene sempre il focus e cattura Tab/Esc/frecce
        resp.request_focus();
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                resp.id,
                EventFilter {
                    tab: true,
                    escape: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                },
            )
        });

        let cols = (rect.width() / cw).floor() as u16;
        let rows = (rect.height() / rh).floor() as u16;
        self.resize(cols, rows);

        self.handle_input(ui);

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::BG);

        let mut parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return,
        };
        parser.set_scrollback(self.scroll);
        let screen = parser.screen();
        // se lo scrollback è più corto del richiesto, riallinea
        self.scroll = screen.scrollback();
        let origin = rect.min;

        for r in 0..self.rows {
            let y = origin.y + r as f32 * rh;
            let mut run = String::new();
            let mut run_col = 0u16;
            let mut run_fg = theme::FG;
            let mut run_bold = false;

            for c in 0..self.cols {
                let cell = match screen.cell(r, c) {
                    Some(cell) => cell,
                    None => continue,
                };
                let mut fg = color_of(cell.fgcolor(), true);
                let mut bg = color_of(cell.bgcolor(), false);
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if bg != theme::BG {
                    let x = origin.x + c as f32 * cw;
                    let w = if cell.is_wide() { cw * 2.0 } else { cw };
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, rh)),
                        0.0,
                        bg,
                    );
                }
                if cell.is_wide_continuation() {
                    continue;
                }
                let txt = cell.contents();
                let ch = if txt.is_empty() { " ".to_string() } else { txt };
                let bold = cell.bold();
                let blank = ch == " ";
                // chiude la run se cambia lo stile
                if !run.is_empty() && (fg != run_fg || bold != run_bold) {
                    flush(&painter, &font, origin.x, y, cw, run_col, &run, run_fg, run_bold);
                    run.clear();
                }
                if run.is_empty() {
                    if blank {
                        continue;
                    }
                    run_col = c;
                    run_fg = fg;
                    run_bold = bold;
                }
                run.push_str(&ch);
            }
            if !run.is_empty() {
                flush(&painter, &font, origin.x, y, cw, run_col, &run, run_fg, run_bold);
            }
        }

        // cursore a blocco
        if self.scroll == 0 && !screen.hide_cursor() {
            let (cr, cc) = screen.cursor_position();
            let cur = Rect::from_min_size(
                Pos2::new(origin.x + cc as f32 * cw, origin.y + cr as f32 * rh),
                Vec2::new(cw, rh),
            );
            painter.rect_filled(cur, 0.0, theme::FG.gamma_multiply(0.75));
        }
    }

    fn handle_input(&mut self, ui: &egui::Ui) {
        let (events, scroll_y) = ui.input(|i| (i.events.clone(), i.raw_scroll_delta.y));

        if scroll_y != 0.0 {
            let lines = (scroll_y / 20.0).round() as isize;
            let new = (self.scroll as isize + lines).max(0) as usize;
            self.scroll = new;
        }

        let app_cursor = self
            .parser
            .lock()
            .map(|p| p.screen().application_cursor())
            .unwrap_or(false);

        for ev in events {
            match ev {
                egui::Event::Text(t) => {
                    self.scroll = 0;
                    self.send(&t);
                }
                egui::Event::Paste(t) => {
                    self.scroll = 0;
                    self.send(&t);
                }
                egui::Event::Copy => self.send("\x03"),
                egui::Event::Cut => self.send("\x18"),
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let arrow = |app: bool, c: char| {
                        if app {
                            format!("\x1bO{c}")
                        } else {
                            format!("\x1b[{c}")
                        }
                    };
                    let seq: Option<String> = match key {
                        Key::Enter => Some("\r".into()),
                        Key::Backspace => Some("\x7f".into()),
                        Key::Tab => Some(if modifiers.shift { "\x1b[Z".into() } else { "\t".into() }),
                        Key::Escape => Some("\x1b".into()),
                        Key::ArrowUp => Some(arrow(app_cursor, 'A')),
                        Key::ArrowDown => Some(arrow(app_cursor, 'B')),
                        Key::ArrowRight => Some(arrow(app_cursor, 'C')),
                        Key::ArrowLeft => Some(arrow(app_cursor, 'D')),
                        Key::Home => Some("\x1b[H".into()),
                        Key::End => Some("\x1b[F".into()),
                        Key::Delete => Some("\x1b[3~".into()),
                        Key::Insert => Some("\x1b[2~".into()),
                        Key::PageUp => Some("\x1b[5~".into()),
                        Key::PageDown => Some("\x1b[6~".into()),
                        Key::F1 => Some("\x1bOP".into()),
                        Key::F2 => Some("\x1bOQ".into()),
                        Key::F3 => Some("\x1bOR".into()),
                        Key::F4 => Some("\x1bOS".into()),
                        Key::F5 => Some("\x1b[15~".into()),
                        Key::F6 => Some("\x1b[17~".into()),
                        Key::F7 => Some("\x1b[18~".into()),
                        Key::F8 => Some("\x1b[19~".into()),
                        Key::F9 => Some("\x1b[20~".into()),
                        Key::F10 => Some("\x1b[21~".into()),
                        Key::F11 => Some("\x1b[23~".into()),
                        Key::F12 => Some("\x1b[24~".into()),
                        k if modifiers.ctrl && !modifiers.alt => ctrl_letter(k),
                        _ => None,
                    };
                    if let Some(s) = seq {
                        self.scroll = 0;
                        self.send(&s);
                    }
                }
                _ => {}
            }
        }
    }
}

fn ctrl_letter(k: Key) -> Option<String> {
    let name = k.name();
    let mut chars = name.chars();
    let c = chars.next()?;
    if chars.next().is_none() && c.is_ascii_alphabetic() {
        let b = (c.to_ascii_lowercase() as u8 - b'a' + 1) as char;
        Some(b.to_string())
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn flush(
    painter: &egui::Painter,
    font: &FontId,
    ox: f32,
    y: f32,
    cw: f32,
    col: u16,
    text: &str,
    color: Color32,
    bold: bool,
) {
    let _ = bold;
    painter.text(
        Pos2::new(ox + col as f32 * cw, y),
        egui::Align2::LEFT_TOP,
        text,
        font.clone(),
        color,
    );
}

fn color_of(c: vt100::Color, is_fg: bool) -> Color32 {
    match c {
        vt100::Color::Default => {
            if is_fg {
                theme::FG
            } else {
                theme::BG
            }
        }
        vt100::Color::Idx(i) => theme::indexed(i),
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

fn default_shell() -> CommandBuilder {
    #[cfg(windows)]
    {
        let mut cmd = CommandBuilder::new("powershell.exe");
        cmd.arg("-NoLogo");
        if let Some(home) = std::env::var_os("USERPROFILE") {
            cmd.cwd(home);
        }
        cmd
    }
    #[cfg(not(windows))]
    {
        let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = CommandBuilder::new(sh);
        cmd.env("TERM", "xterm-256color");
        if let Some(home) = std::env::var_os("HOME") {
            cmd.cwd(home);
        }
        cmd
    }
}
