use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use std::io::Cursor;

// Gli stessi suoni dell'originale eDEX-UI (assets/audio), con gli stessi volumi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Snd {
    Stdout,
    Stdin,
    Folder,
    Granted,
    Keyboard,
    Theme,
    Expand,
    Panels,
    Scan,
}

pub struct Audio {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
}

impl Audio {
    pub fn new() -> Self {
        match OutputStream::try_default() {
            Ok((s, h)) => Self { _stream: Some(s), handle: Some(h) },
            Err(_) => Self { _stream: None, handle: None },
        }
    }

    fn data(s: Snd) -> (&'static [u8], f32) {
        match s {
            Snd::Stdout => (include_bytes!("../assets/audio/stdout.wav"), 0.4),
            Snd::Stdin => (include_bytes!("../assets/audio/stdin.wav"), 0.4),
            Snd::Folder => (include_bytes!("../assets/audio/folder.wav"), 1.0),
            Snd::Granted => (include_bytes!("../assets/audio/granted.wav"), 1.0),
            Snd::Keyboard => (include_bytes!("../assets/audio/keyboard.wav"), 1.0),
            Snd::Theme => (include_bytes!("../assets/audio/theme.wav"), 1.0),
            Snd::Expand => (include_bytes!("../assets/audio/expand.wav"), 1.0),
            Snd::Panels => (include_bytes!("../assets/audio/panels.wav"), 1.0),
            Snd::Scan => (include_bytes!("../assets/audio/scan.wav"), 1.0),
        }
    }

    pub fn play(&self, s: Snd) {
        let Some(h) = &self.handle else { return };
        let (bytes, vol) = Self::data(s);
        if let Ok(dec) = Decoder::new(Cursor::new(bytes)) {
            let _ = h.play_raw(dec.convert_samples::<f32>().amplify(vol));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_wavs_decode() {
        for s in [Snd::Stdout, Snd::Stdin, Snd::Folder, Snd::Granted, Snd::Keyboard, Snd::Theme, Snd::Expand, Snd::Panels, Snd::Scan] {
            let (b, _) = Audio::data(s);
            let d = Decoder::new(Cursor::new(b)).unwrap_or_else(|e| panic!("{s:?}: {e}"));
            let n = d.count();
            assert!(n > 1000, "{s:?} troppo corto: {n}");
        }
    }
}
