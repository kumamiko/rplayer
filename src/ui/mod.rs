mod layout;
mod playlist;
mod lyrics;
mod statusbar;
mod search;
mod message;

pub use playlist::*;
pub use lyrics::*;
pub use statusbar::*;
pub use search::*;
pub use message::*;

use crate::app::App;
use crate::lyrics::LyricsManager;
use ratatui::Frame;

pub struct Ui<'a> {
    app: &'a App,
    lyrics: &'a LyricsManager,
}

impl<'a> Ui<'a> {
    pub fn new(app: &'a App, lyrics: &'a LyricsManager) -> Self {
        Self { app, lyrics }
    }
    
    pub fn render(&self, f: &mut Frame) {
        let chunks = layout::create_layout(f.area());
        
        // Playlist (left side)
        let playlist = PlaylistWidget::new(self.app);
        f.render_stateful_widget(playlist, chunks.playlist, &mut ());
        
        // Lyrics (right side)
        let lyrics_widget = LyricsWidget::new(self.lyrics, self.app.current_pos);
        f.render_widget(lyrics_widget, chunks.lyrics);
        
        // Message bar (above status bar)
        let message = MessageWidget::new(self.app);
        f.render_widget(message, chunks.message);
        
        // Status bar (bottom)
        let statusbar = StatusbarWidget::new(self.app);
        f.render_widget(statusbar, chunks.statusbar);
        
        // Search bar (when in search mode)
        if self.app.mode == crate::app::Mode::Search {
            let search = SearchWidget::new(&self.app.search_query);
            f.render_widget(search, chunks.statusbar);
        }
    }
}
