use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct LayoutChunks {
    pub playlist: Rect,
    pub lyrics: Rect,
    pub message: Rect,
    pub statusbar: Rect,
}

pub fn create_layout(area: Rect) -> LayoutChunks {
    // Split into content area and status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),       // Content area
            Constraint::Length(1),    // Message bar
            Constraint::Length(1),    // Status bar
        ])
        .split(area);
    
    // Split content into playlist (top) and lyrics (bottom, fixed 3 lines)
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),       // Playlist
            Constraint::Length(4),    // Lyrics (karaoke style, 3 lines max)
        ])
        .split(main_chunks[0]);
    
    LayoutChunks {
        playlist: content_chunks[0],
        lyrics: content_chunks[1],
        message: main_chunks[1],
        statusbar: main_chunks[2],
    }
}
