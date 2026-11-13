use crate::settings::Settings;
use crate::theme;
use eframe::egui::{self, Color32, EventFilter, FontId, Key, Pos2, Rect, Sense, Vec2};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
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
    child: Box<dyn Child + Send + Sync>,
    sel: Option<((u16, u16), (u16, u16))>,
}

impl Terminal {
    pub fn new(ctx: &egui::Context, st: &Settings) -> std::io::Result<Self> {
        let (cols, rows) = (100u16, 30u16);
        let pair = native_pty_system()
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let cmd = default_shell(st);
        let child = pair
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

        Ok(Self { parser, writer, master: pair.master, cols, rows, scroll: 0, out, child, sel: None })
    }

    /// Testo della selezione corrente (per Ctrl+Shift+C).
    pub fn selection_text(&self) -> Option<String> {
        let (a, b) = self.sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) })?;
        let p = self.parser.lock().ok()?;
        let screen = p.screen();
        let mut out = String::new();
        for r in a.0..=b.0 {
            let c0 = if r == a.0 { a.1 } else { 0 };
            let c1 = if r == b.0 { b.1 } else { self.cols.saturating_sub(1) };
            let mut line = String::new();
            for c in c0..=c1 {
                if let Some(cell) = screen.cell(r, c) {
                    if !cell.is_wide_continuation() {
                        let t = cell.contents();
                        line.push_str(if t.is_empty() { " " } else { &t });
                    }
                }
            }
            out.push_str(line.trim_end());
            if r != b.0 {
                out.push('\n');
            }
        }
        if out.is_empty() { None } else { Some(out) }
    }

    /// True se il cursore è subito dopo un prompt di password (ssh, sudo, passphrase...).
    pub fn password_prompt(&self) -> bool {
        let Ok(p) = self.parser.lock() else { return false };
        let screen = p.screen();
        let (row, col) = screen.cursor_position();
        let mut line = String::new();
        for c in 0..col {
            if let Some(cell) = screen.cell(row, c) {
                if !cell.is_wide_continuation() {
                    let t = cell.contents();
                    line.push_str(if t.is_empty() { " " } else { &t });
                }
            }
        }
        let l = line.trim_end().to_lowercase();
        (l.ends_with(':') || l.ends_with('?'))
            && ["password", "passphrase", "passcode"].iter().any(|k| l.contains(k))
    }

    /// True se è arrivato output dall'ultima chiamata (per il suono stdout).
    pub fn take_output(&self) -> bool {
        self.out.swap(false, Ordering::Relaxed)
    }

    /// True quando la shell è terminata (exit).
    pub fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Click destro: incolla il testo della clipboard nel terminale.
    fn paste_clipboard(&mut self) {
        let Ok(mut cb) = arboard::Clipboard::new() else { return };
        let Ok(text) = cb.get_text() else { return };
        if text.is_empty() {
            return;
        }
        let text = text.replace("\r\n", "\r").replace('\n', "\r");
        let bracketed = self.parser.lock().map(|p| p.screen().bracketed_paste()).unwrap_or(false);
        self.scroll = 0;
        if bracketed {
            self.send(&format!("\x1b[200~{text}\x1b[201~"));
        } else {
            self.send(&text);
        }
    }

    pub fn send(&mut self, s: &str) {
        self.sel = None;
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

    pub fn ui(&mut self, ui: &mut egui::Ui, rect: Rect, id: egui::Id, size: f32, show_cursor: bool, input: bool) {
        let font = theme::font(size, "mono");
        let (cw, rh) = ui.fonts(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let resp = ui.interact(rect, id, Sense::click_and_drag());

        // il terminale tiene sempre il focus e cattura Tab/Esc/frecce
        if input {
            resp.request_focus();
        }
        if resp.secondary_clicked() {
            self.paste_clipboard();
        }
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

        // selezione con il mouse (trascinando); Ctrl+Shift+C copia
        {
            let (cols_f, rows_f) = (self.cols as f32, self.rows as f32);
            let cell_at = |pos: Pos2| -> (u16, u16) {
                (
                    ((pos.y - rect.top()) / rh).floor().clamp(0.0, rows_f - 1.0) as u16,
                    ((pos.x - rect.left()) / cw).floor().clamp(0.0, cols_f - 1.0) as u16,
                )
            };
            if resp.drag_started_by(egui::PointerButton::Primary) {
                if let Some(p) = resp.interact_pointer_pos() {
                    let c = cell_at(p);
                    self.sel = Some((c, c));
                }
            } else if resp.dragged_by(egui::PointerButton::Primary) {
                if let (Some(p), Some((a, _))) = (resp.interact_pointer_pos(), self.sel) {
                    self.sel = Some((a, cell_at(p)));
                }
            }
            if resp.clicked_by(egui::PointerButton::Primary) {
                self.sel = None;
            }
        }

        if input {
            self.handle_input(ui);
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::term_bg());

        let mut parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return,
        };
        parser.set_scrollback(self.scroll);
        let screen = parser.screen();
        // se lo scrollback è più corto del richiesto, riallinea
        self.scroll = screen.scrollback();
        let origin = rect.min;
        let sel = self.sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });

        for r in 0..self.rows {
            let y = origin.y + r as f32 * rh;
            let mut run = String::new();
            let mut run_col = 0u16;
            let mut run_fg = theme::term_fg();
            let mut run_bold = false;

            for c in 0..self.cols {
                let cell = match screen.cell(r, c) {
                    Some(cell) => cell,
                    None => continue,
                };
                if let Some((a, b)) = sel {
                    if (r, c) >= a && (r, c) <= b {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(origin.x + c as f32 * cw, y), Vec2::new(cw, rh)),
                            0.0,
                            theme::fg_a(0.3),
                        );
                    }
                }
                let mut fg = color_of(cell.fgcolor(), true);
                let mut bg = color_of(cell.bgcolor(), false);
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if bg != theme::term_bg() {
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
        if show_cursor && self.scroll == 0 && !screen.hide_cursor() {
            let (cr, cc) = screen.cursor_position();
            let cur = Rect::from_min_size(
                Pos2::new(origin.x + cc as f32 * cw, origin.y + cr as f32 * rh),
                Vec2::new(cw, rh),
            );
            painter.rect_filled(cur, 0.0, theme::cursor().gamma_multiply(0.75));
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
                egui::Event::Copy => {
                    if ui.input(|i| i.modifiers.shift) {
                        if let Some(t) = self.selection_text() {
                            ui.ctx().copy_text(t);
                        }
                    } else {
                        self.send("\x03");
                    }
                }
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
                theme::term_fg()
            } else {
                theme::term_bg()
            }
        }
        vt100::Color::Idx(i) => theme::indexed(i),
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

fn default_shell(st: &Settings) -> CommandBuilder {
    let prog = if st.shell.trim().is_empty() {
        if cfg!(windows) { "powershell.exe".to_string() } else { std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()) }
    } else {
        st.shell.trim().to_string()
    };
    let mut cmd = CommandBuilder::new(&prog);
    let args = if st.shell_args.trim().is_empty() {
        if cfg!(windows) && prog.to_lowercase().contains("powershell") { "-NoLogo".to_string() } else { String::new() }
    } else {
        st.shell_args.clone()
    };
    for a in args.split_whitespace() {
        cmd.arg(a);
    }
    cmd.env("TERM", "xterm-256color");
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    if !st.cwd.trim().is_empty() && std::path::Path::new(st.cwd.trim()).is_dir() {
        cmd.cwd(st.cwd.trim());
    } else if let Some(h) = home {
        cmd.cwd(h);
    }
    cmd
}
