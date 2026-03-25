mod layout;
mod playlist;
mod lyrics;
mod statusbar;
mod search;
mod message;
mod help;

pub use playlist::*;
pub use lyrics::*;
pub use statusbar::*;
pub use search::*;
pub use message::*;
pub use help::*;

use crate::app::App;
use crate::lyrics::LyricsManager;
use ratatui::Frame;

pub struct Ui<'a> {
    app: &'a mut App,
    lyrics: &'a LyricsManager,
}

impl<'a> Ui<'a> {
    pub fn new(app: &'a mut App, lyrics: &'a LyricsManager) -> Self {
        Self { app, lyrics }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let chunks = layout::create_layout(f.area());
        let theme_border = self.app.theme_color().unwrap_or(ratatui::style::Color::Cyan);
        let theme_title = self.app.theme_color_bright().unwrap_or(ratatui::style::Color::Cyan);

        // Update visible height for dynamic scroll/page
        self.app.playlist_visible_height = chunks.playlist.height.saturating_sub(2) as usize;
        
        // Playlist (left side)
        let playlist = PlaylistWidget::new(self.app);
        f.render_stateful_widget(playlist, chunks.playlist, &mut ());
        
        // Lyrics (right side)
        let lyrics_widget = LyricsWidget::new(self.lyrics, self.app.current_pos, theme_border, theme_title);
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
        
        // Theme color input (when in theme color mode)
        if self.app.mode == crate::app::Mode::ThemeColor {
            let bg = ratatui::style::Style::default().bg(ratatui::style::Color::Magenta).fg(ratatui::style::Color::White);
            let mut spans = vec![
                ratatui::text::Span::styled(" THEME: ", bg),
                ratatui::text::Span::styled(&self.app.theme_color_input, bg),
                ratatui::text::Span::styled("█", bg),
            ];
            let used = 8 + self.app.theme_color_input.len() + 1;
            if (chunks.statusbar.width as usize) > used {
                spans.push(ratatui::text::Span::styled(
                    " ".repeat((chunks.statusbar.width as usize) - used),
                    bg,
                ));
            }
            let paragraph = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(spans));
            f.render_widget(paragraph, chunks.statusbar);
        }
        
        // Help popup (when in help mode)
        if self.app.mode == crate::app::Mode::Help {
            let help = HelpWidget::new(theme_border, theme_title);
            f.render_widget(help, f.area());
        }
    }
}
