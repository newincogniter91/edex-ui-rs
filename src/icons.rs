use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use regex::{Regex, RegexSetBuilder};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

// Icone: Font Awesome Free 5.15 (CC BY 4.0), come nell'originale eDEX-UI,
// pre-rasterizzate a 64x64 (solo canale alpha) e compresse.
const BLOB: &[u8] = include_bytes!("../assets/icons.bin");
const INDEX: &str = include_str!("../assets/icons_index.json");
const RULES: &str = include_str!("../assets/icon_rules.json");

struct Matcher {
    set: regex::RegexSet,
    names: Vec<usize>, // indice icona per ogni regola
}

pub struct Icons {
    size: usize,
    index: HashMap<String, usize>,
    alpha: Vec<u8>,
    textures: HashMap<usize, TextureHandle>,
    matcher: Arc<Mutex<Option<Matcher>>>,
    cache: HashMap<String, usize>,
}

impl Icons {
    pub fn new() -> Self {
        let mut alpha = Vec::new();
        let _ = flate2::read::ZlibDecoder::new(BLOB).read_to_end(&mut alpha);
        let idx: serde_json::Value = serde_json::from_str(INDEX).unwrap_or(serde_json::Value::Null);
        let size = idx["size"].as_u64().unwrap_or(64) as usize;
        let mut index = HashMap::new();
        if let Some(a) = idx["names"].as_array() {
            for (i, n) in a.iter().enumerate() {
                if let Some(s) = n.as_str() {
                    index.insert(s.to_string(), i);
                }
            }
        }

        // le regole (oltre 1800 regex) si compilano in background
        let matcher: Arc<Mutex<Option<Matcher>>> = Arc::new(Mutex::new(None));
        let m2 = matcher.clone();
        let index2 = index.clone();
        std::thread::spawn(move || {
            let rules: Vec<[String; 3]> = serde_json::from_str(RULES).unwrap_or_default();
            let mut pats = Vec::new();
            let mut names = Vec::new();
            for [pat, flags, name] in rules {
                let p = if flags.contains('i') { format!("(?i){pat}") } else { pat };
                if Regex::new(&p).is_ok() {
                    if let Some(i) = index2.get(&name) {
                        pats.push(p);
                        names.push(*i);
                    }
                }
            }
            if let Ok(set) = RegexSetBuilder::new(&pats).size_limit(512 << 20).dfa_size_limit(256 << 20).build() {
                if let Ok(mut g) = m2.lock() {
                    *g = Some(Matcher { set, names });
                }
            }
        });

        Self { size, index, alpha, textures: HashMap::new(), matcher, cache: HashMap::new() }
    }

    pub fn core(&self, name: &str) -> usize {
        *self.index.get(name).or_else(|| self.index.get("file")).unwrap_or(&0)
    }

    /// Icona per un file/cartella in base al nome (stesse regole dell'originale).
    pub fn for_entry(&mut self, name: &str, is_dir: bool) -> usize {
        let fallback = self.core(if is_dir { "dir" } else { "file" });
        if let Some(i) = self.cache.get(name) {
            return *i;
        }
        let found = match self.matcher.lock() {
            Ok(g) => g.as_ref().map(|m| m.set.matches(name).into_iter().min().map(|r| m.names[r])),
            Err(_) => None,
        };
        match found {
            Some(Some(i)) => {
                self.cache.insert(name.to_string(), i);
                i
            }
            Some(None) => {
                self.cache.insert(name.to_string(), fallback);
                fallback
            }
            None => fallback, // matcher non ancora pronto: non memorizzare
        }
    }

    pub fn texture(&mut self, ctx: &egui::Context, i: usize) -> Option<egui::TextureId> {
        if !self.textures.contains_key(&i) {
            let n = self.size * self.size;
            let data = self.alpha.get(i * n..(i + 1) * n)?;
            let pixels: Vec<Color32> = data.iter().map(|a| Color32::from_rgba_unmultiplied(255, 255, 255, *a)).collect();
            let img = ColorImage { size: [self.size, self.size], pixels };
            let t = ctx.load_texture(format!("icon{i}"), img, TextureOptions::LINEAR);
            self.textures.insert(i, t);
        }
        self.textures.get(&i).map(|t| t.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcher_builds_and_matches() {
        let mut ic = Icons::new();
        for _ in 0..200 {
            if ic.matcher.lock().unwrap().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(ic.matcher.lock().unwrap().is_some(), "RegexSet non compilato");
        let generic = ic.core("file");
        let rs = ic.for_entry("main.rs", false);
        let py = ic.for_entry("script.py", false);
        assert_ne!(rs, generic);
        assert_ne!(py, generic);
        assert_ne!(rs, py);
    }
}
