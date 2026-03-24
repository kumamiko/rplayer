mod parser;

pub use parser::*;

use std::collections::BTreeMap;
use std::time::Duration;

pub struct LyricsManager {
    lyrics: BTreeMap<Duration, String>,
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
    
    fn load_lyrics(&self, song_path: &str) -> BTreeMap<Duration, String> {
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
        use lofty::probe::Probe;
        use lofty::file::TaggedFileExt;
        use lofty::tag::{ItemKey, ItemValue};
        
        let path = std::path::Path::new(song_path);
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file.primary_tag()?;
        
        // Look for lyrics in tag items (ItemKey::Lyrics covers USLT, ©lyr, LYRICS, etc.)
        for item in tag.items() {
            if *item.key() == ItemKey::Lyrics {
                if let ItemValue::Text(text) = item.value() {
                    return Some(text.clone());
                }
            }
        }
        
        None
    }
    
    /// Get current and next lyrics line for karaoke display
    pub fn get_current_and_next(&self, position: Duration) -> (Option<String>, Option<String>) {
        let mut current: Option<String> = None;
        let mut next: Option<String> = None;
        let mut found_current = false;
        
        for (time, line) in self.lyrics.iter() {
            if *time <= position {
                current = Some(line.clone());
                found_current = true;
            } else if found_current && next.is_none() {
                next = Some(line.clone());
                break;
            } else if !found_current {
                // Before any lyrics start
                next = Some(line.clone());
                break;
            }
        }
        
        (current, next)
    }
}
