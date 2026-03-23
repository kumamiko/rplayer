mod parser;
mod sync;

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
        
        self.lyrics.clear();
        self.loaded_path = Some(song_path.to_string());
        
        // Try to find .lrc file (same name as song but with .lrc extension)
        let lrc_path = format!("{}.lrc", song_path.rsplit_once('.').map(|(p, _)| p).unwrap_or(song_path));
        
        if let Ok(content) = std::fs::read_to_string(&lrc_path) {
            self.lyrics = parse_lrc(&content);
        }
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
    
    pub fn is_loaded(&self) -> bool {
        !self.lyrics.is_empty()
    }
}
