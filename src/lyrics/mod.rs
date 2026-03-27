mod parser;

pub use parser::*;

use std::collections::BTreeMap;
use std::time::Duration;

pub struct LyricsManager {
    lyrics: BTreeMap<Duration, LyricsLine>,
    loaded_path: Option<String>,
}

impl Default for LyricsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsManager {
    pub fn new() -> Self {
        Self {
            lyrics: BTreeMap::new(),
            loaded_path: None,
        }
    }

    pub fn load(&mut self, song_path: &str) {
        // Don't reload if same song
        if self.loaded_path.as_deref() == Some(song_path) {
            return;
        }

        let new_lyrics = self.load_lyrics(song_path);
        self.lyrics = new_lyrics;
        self.loaded_path = Some(song_path.to_string());
    }

    fn load_lyrics(&self, song_path: &str) -> BTreeMap<Duration, LyricsLine> {
        // 1. Try external .lrc file first
        let lrc_path = format!("{}.lrc", song_path.rsplit_once('.').map(|(p, _)| p).unwrap_or(song_path));

        if let Ok(content) = std::fs::read_to_string(&lrc_path) {
            let lyrics = parse_lrc(&content);
            if !lyrics.is_empty() {
                return lyrics;
            }
        }

        // 2. Try embedded lyrics from audio file (must be LRC format)
        if let Some(embedded) = self.load_embedded_lyrics(song_path) {
            let lyrics = parse_lrc(&embedded);
            if !lyrics.is_empty() {
                return lyrics;
            }
        }

        // 3. No lyrics available
        BTreeMap::new()
    }

    fn load_embedded_lyrics(&self, song_path: &str) -> Option<String> {
        use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
        use symphonia::core::probe::Hint;
        use symphonia::default::get_probe;

        let path = std::path::Path::new(song_path);
        let file = std::fs::File::open(path).ok()?;
        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let mut probed = get_probe()
            .format(&hint, mss, &Default::default(), &MetadataOptions::default())
            .ok()?;
        let mut format_reader = probed.format;

        // 1. Check probed metadata first (e.g., ID3v2 tags in MP3 files)
        if let Some(probed_meta) = probed.metadata.get() {
            if let Some(rev) = probed_meta.current() {
                for tag in rev.tags() {
                    if tag.std_key == Some(StandardTagKey::Lyrics) {
                        if let Value::String(text) = &tag.value {
                            return Some(text.clone());
                        }
                    }
                }
            }
        }

        // 2. Check container metadata (e.g., Vorbis Comments in FLAC)
        let mut metadata = format_reader.metadata();
        while !metadata.is_latest() {
            metadata.pop();
        }

        if let Some(rev) = metadata.current() {
            for tag in rev.tags() {
                if tag.std_key == Some(StandardTagKey::Lyrics) {
                    if let Value::String(text) = &tag.value {
                        return Some(text.clone());
                    }
                }
            }
        }

        None
    }

    /// Get current and next lyrics line for karaoke display
    /// Returns (current_line, next_line) as references
    pub fn get_current_and_next(&self, position: Duration) -> (Option<&LyricsLine>, Option<&LyricsLine>) {
        let mut current: Option<&LyricsLine> = None;
        let mut next: Option<&LyricsLine> = None;
        let mut found_current = false;

        for (_time, line) in self.lyrics.iter() {
            if line.time() <= position {
                current = Some(line);
                found_current = true;
            } else if found_current && next.is_none() {
                next = Some(line);
                break;
            } else if !found_current {
                next = Some(line);
                break;
            }
        }

        (current, next)
    }
}
