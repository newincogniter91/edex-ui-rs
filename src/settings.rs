use crate::data::THEMES;
use crate::theme::{self, Palette};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Impostazioni salvate in settings.json (stesse chiavi dell'originale, dove esistono).
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub shell: String,
    pub shell_args: String,
    pub cwd: String,
    pub keyboard: String,
    pub theme: String,
    pub term_font_size: f32,
    pub audio: bool,
    pub audio_volume: f32,
    pub disable_feedback_audio: bool,
    pub clock_hours: u8,
    pub ping_addr: String,
    pub geoip: bool,
    pub nointro: bool,
    pub nocursor: bool,
    pub allow_windowed: bool,
    pub hide_dotfiles: bool,
    pub fs_list_view: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shell: if cfg!(windows) { "powershell.exe".into() } else { std::env::var("SHELL").unwrap_or_else(|_| "bash".into()) },
            shell_args: String::new(),
            cwd: String::new(),
            keyboard: "en-US".into(),
            theme: "tron".into(),
            term_font_size: 15.0,
            audio: true,
            audio_volume: 1.0,
            disable_feedback_audio: false,
            clock_hours: 24,
            ping_addr: "1.1.1.1".into(),
            geoip: true,
            nointro: false,
            nocursor: false,
            allow_windowed: false,
            hide_dotfiles: false,
            fs_list_view: false,
        }
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("edex-ui-rs")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(config_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("settings.json"), s);
        }
    }
}

// ------------------------------------------------------------------ temi

fn hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() >= 6 {
        let v = u32::from_str_radix(&s[..6], 16).ok()?;
        return Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    None
}

pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|(n, _)| *n).collect()
}

/// Costruisce la palette dal JSON del tema e la rende attiva.
pub fn apply_theme(name: &str) {
    let raw = THEMES.iter().find(|(n, _)| *n == name).or_else(|| THEMES.first()).map(|(_, r)| *r).unwrap_or("{}");
    let v: serde_json::Value = serde_json::from_str(raw).unwrap_or(serde_json::Value::Null);
    let mut p: Palette = theme::DEFAULT;
    let col = &v["colors"];
    if let (Some(r), Some(g), Some(b)) = (col["r"].as_u64(), col["g"].as_u64(), col["b"].as_u64()) {
        p.fg = [r as u8, g as u8, b as u8];
        p.term_fg = p.fg;
        p.cursor = p.fg;
    }
    if let Some(x) = col["light_black"].as_str().and_then(hex) {
        p.bg = x;
        p.term_bg = x;
    }
    if let Some(x) = col["grey"].as_str().and_then(hex) {
        p.grey = x;
    }
    let t = &v["terminal"];
    if let Some(x) = t["foreground"].as_str().and_then(hex) {
        p.term_fg = x;
    }
    if let Some(x) = t["background"].as_str().and_then(hex) {
        p.term_bg = x;
    }
    if let Some(x) = t["cursor"].as_str().and_then(hex) {
        p.cursor = x;
    }
    theme::set_palette(p);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::IntoColor;

    #[test]
    fn settings_roundtrip_and_defaults() {
        let s = Settings::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&j).unwrap();
        assert!(s == back);
        // file parziale: i campi mancanti prendono il valore di default
        let p: Settings = serde_json::from_str(r#"{"theme":"nord"}"#).unwrap();
        assert_eq!(p.theme, "nord");
        assert_eq!(p.clock_hours, 24);
    }

    #[test]
    fn all_themes_and_layouts_parse() {
        for (n, raw) in THEMES {
            let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_else(|e| panic!("tema {n}: {e}"));
            assert!(v["colors"]["r"].is_number(), "tema {n} senza colori");
        }
        assert_eq!(theme_names().len(), 21);
        for (n, raw) in crate::data::LAYOUTS {
            let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_else(|e| panic!("layout {n}: {e}"));
            assert!(v["row_numbers"].is_array(), "layout {n}");
        }
        apply_theme("nord");
        assert_ne!(theme::fg(), theme::DEFAULT.fg.into_color());
    }
}
