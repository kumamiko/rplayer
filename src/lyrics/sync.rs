use std::time::Duration;

/// Synchronize lyrics with playback position
pub struct LyricsSync {
    current_index: usize,
}

impl Default for LyricsSync {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsSync {
    pub fn new() -> Self {
        Self { current_index: 0 }
    }
    
    /// Find the index of the current lyric line based on position
    pub fn find_current_index(lines: &[(Duration, String)], position: Duration) -> usize {
        let mut idx = 0;
        for (i, (time, _)) in lines.iter().enumerate() {
            if *time <= position {
                idx = i;
            } else {
                break;
            }
        }
        idx
    }
    
    /// Get visible range of lyrics (for scrolling)
    pub fn get_visible_range(total: usize, current: usize, visible_height: usize) -> (usize, usize) {
        let half = visible_height / 2;
        
        let start = if current > half {
            current - half
        } else {
            0
        };
        
        let end = (start + visible_height).min(total);
        
        (start, end)
    }
}
